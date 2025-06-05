use std::{
    borrow::Cow,
    ffi::OsString,
    marker::PhantomData,
    path::PathBuf,
    str::FromStr,
sync::{Arc, Mutex},
};

use clap::Parser;
use reedline::{
    default_emacs_keybindings, Emacs, FileBackedHistory, KeyModifiers, MenuBuilder, Prompt,
    PromptHistorySearch, PromptHistorySearchStatus, Reedline, ReedlineEvent, ReedlineMenu, Signal,
    Span,
};
use shlex::Shlex;
use zencan_cli::command::{Cli, Commands, NmtAction};
use zencan_client::{open_socketcan, BusManager, NodeConfig};

#[derive(Parser)]
struct Args {
    /// The CAN socket to connect to (e.g. 'can0' or 'van0')
    socket: String,
}

struct ZencanPrompt {
    socket: String,
    node_state: Arc<Mutex<usize>>,
}

impl ZencanPrompt {
    pub fn new<S: Into<String>>(socket: S, node_state: Arc<Mutex<usize>>) -> Self {
        let socket = socket.into();
        Self { socket, node_state }
    }
}

impl Prompt for ZencanPrompt {
    fn render_prompt_left(&self) -> std::borrow::Cow<str> {
        Cow::from(&self.socket)
    }

    fn render_prompt_right(&self) -> std::borrow::Cow<str> {
        let node_state = self.node_state.lock().unwrap();
        Cow::Owned(format!("Nodes: {}", node_state))
    }

    fn render_prompt_indicator(
        &self,
        _prompt_mode: reedline::PromptEditMode,
    ) -> std::borrow::Cow<str> {
        Cow::Borrowed(">")
    }

    fn render_prompt_multiline_indicator(&self) -> std::borrow::Cow<str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> std::borrow::Cow<str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!(
            "({}reverse-search: {}) ",
            prefix, history_search.term
        ))
    }
}

struct Completer<C: Parser + Send + Sync + 'static> {
    c_phantom: PhantomData<C>,
}
impl<C: Parser + Send + Sync + 'static> Completer<C> {
    pub fn new() -> Self {
        Self {
            c_phantom: PhantomData::<C>
        }
    }
}

impl<C: Parser + Send + Sync + 'static> reedline::Completer for Completer<C> {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<reedline::Suggestion> {
        let mut cmd = C::command();
        //let mut cmd = clap_complete::engine::complete()::CompleteCommand::augment_subcommands(cmd);

        let args = Shlex::new(line);
        let mut args = std::iter::once("".to_owned())
            .chain(args)
            .map(OsString::from)
            .collect::<Vec<_>>();
        if line.ends_with(' ') {
            args.push(OsString::new());
        }

        let arg_index = args.len() - 1;
        let span = Span::new(pos - args[arg_index].len(), pos);

        if line.is_empty() {
            return cmd
                .get_subcommands()
                .map(|cmd| reedline::Suggestion {
                    value: cmd.get_name().to_owned(),
                    description: cmd.get_after_help().map(|x| x.to_string()),
                    style: None,
                    extra: None,
                    span,
                    append_whitespace: true,
                })
                .collect();
        }
        let Ok(candidates) = clap_complete::engine::complete(
            &mut cmd,
            args,
            arg_index,
            PathBuf::from_str(".").ok().as_deref(),
        ) else {
            return vec![];
        };
        candidates
            .into_iter()
            .map(|c| reedline::Suggestion {
                value: c.get_value().to_string_lossy().into_owned(),
                description: c.get_help().map(|x| x.to_string()),
                style: None,
                extra: None,
                span,
                append_whitespace: false,
            })
            .collect()
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    let node_state = Arc::new(Mutex::new(0));
    let prompt = ZencanPrompt::new(&args.socket, node_state.clone());

    let (tx, rx) = open_socketcan(&args.socket).expect("Failed to open bus socket");
    let mut manager = BusManager::new(tx, rx);

    let completion_menu = Box::new(
        reedline::IdeMenu::default()
            .with_default_border()
            .with_name("completion_menu"),
    );
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        reedline::KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    let edit_mode = Box::new(Emacs::new(keybindings));

    let mut rl = Reedline::create()
        .with_completer(Box::new(Completer::<Cli>::new()))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_history(Box::new(
            FileBackedHistory::with_file(10000, "/tmp/zencan-cli-history".into()).unwrap(),
        ))
        .with_edit_mode(edit_mode);

    loop {
        let nodes = manager.node_list().await;
        *node_state.lock().unwrap() = nodes.len();
        let line = match rl.read_line(&prompt) {
            Ok(Signal::Success(line)) => line,
            Ok(Signal::CtrlC) => continue,
            Ok(Signal::CtrlD) => {
                println!("Exiting...");
                break;
            }
            Err(e) => panic!("Reedline error: {e}"),
        };

        let cmd = match shlex::split(&line) {
            Some(split) => {
                match Cli::try_parse_from(
                    std::iter::once("").chain(split.iter().map(String::as_str)),
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        println!("{e}");
                        continue;
                    }
                }
            }
            None => {
                panic!("shlex!");
            }
        };

        match cmd.command {
            Commands::Scan => {
                let nodes = manager.scan_nodes().await;
                for n in &nodes {
                    println!("{n}");
                }
            }
            Commands::Info => {
                let nodes = manager.node_list().await;
                for n in &nodes {
                    println!("{n}");
                }
            }
            Commands::Nmt(cmd) => match cmd.action {
                NmtAction::ResetApp => manager.nmt_reset_app(cmd.node.raw()).await,
                NmtAction::ResetComms => manager.nmt_reset_comms(cmd.node.raw()).await,
                NmtAction::Start => manager.nmt_start(cmd.node.raw()).await,
                NmtAction::Stop => manager.nmt_stop(cmd.node.raw()).await,
            },
            Commands::LoadConfig(args) => {
                let config = match NodeConfig::load_from_file(&args.path) {
                    Ok(c) => c,
                    Err(e) => {
                        println!("Error reading config file: ");
                        println!("{e}");
                        return;
                    }
                };
                let mut client = manager.sdo_client(args.node_id);
                for (pdo_num, cfg) in config.tpdos() {
                    if let Err(e) = client.configure_tpdo(*pdo_num, cfg).await {
                        println!("Error configuring TPDO {pdo_num}:");
                        println!("{e}");
                        continue;
                    }
                }
                for (pdo_num, cfg) in config.rpdos() {
                    if let Err(e) = client.configure_rpdo(*pdo_num, cfg).await {
                        println!("Error configuring RPDO {pdo_num}:");
                        println!("{e}");
                        continue;
                    }
                }
                for store in config.stores() {
                    if let Err(e) = client
                        .download(store.index, store.sub, &store.raw_value())
                        .await
                    {
                        println!(
                            "Error storing object at index {:04X} sub {}: {e}",
                            store.index, store.sub
                        );
                        continue;
                    }
                }
            }
            Commands::Lss(_lss_commands) => todo!(),
        }
    }
}
