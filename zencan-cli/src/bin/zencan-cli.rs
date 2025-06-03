use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

use clap::Parser;
use clap_repl::reedline::{
    FileBackedHistory, Prompt, PromptHistorySearch, PromptHistorySearchStatus,
};
use clap_repl::ClapEditor;
use zencan_client::{open_socketcan, BusManager};

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
        _prompt_mode: clap_repl::reedline::PromptEditMode,
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
        // NOTE: magic strings, given there is logic on how these compose I am not sure if it
        // is worth extracting in to static constant
        Cow::Owned(format!(
            "({}reverse-search: {}) ",
            prefix, history_search.term
        ))
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

    let rl = ClapEditor::<zencan_cli::command::Cli>::builder()
        .with_prompt(Box::new(prompt))
        .with_editor_hook(|reed| {
            // Do custom things with `Reedline` instance here
            reed.with_history(Box::new(
                FileBackedHistory::with_file(10000, "/tmp/zencan-cli-history".into()).unwrap(),
            ))
        })
        .build();

    rl.repl_async(async |command| {
        match command.command {
            zencan_cli::command::Commands::Scan => {
                let nodes = manager.scan_nodes().await;
                for n in &nodes {
                    println!("{n}");
                }
            }
            zencan_cli::command::Commands::Info => {
                let nodes = manager.node_list().await;
                for n in &nodes {
                    println!("{n}");
                }
            }
            zencan_cli::command::Commands::Lss(_lss_commands) => todo!(),
        }
        println!("{:?}", command);
    })
    .await;
}
