use crossbeam::atomic::AtomicCell;
use manual_node::*;

use zencan_common::{objects::{Context}, sdo::AbortCode};
use zencan_node::node::Node;

#[derive(Debug, Default)]
struct CallbackContext {
    read: AtomicCell<u16>,
    write: AtomicCell<u16>,
}


// impl Context for CallbackContext {
//     fn as_any<'a, 'b: 'a>(&'b self) -> &'a dyn std::any::Any {
//         &self
//     }
// }

fn test_pdo_rx() {

    // fn read_callback( ctx: &Option<&dyn Context>,
    //     object: &Object,
    //     sub: u8,
    //     offset: usize,
    //     buf: &mut [u8]
    // ) -> Result<(), AbortCode> {
    //     let ctx = ctx.unwrap().as_any().downcast_ref::<CallbackContext>().unwrap();
    //     ctx.read.fetch_add(1);
    //     Ok(())
    // }

    // fn write_callback(
    //     ctx: &Option<&dyn Context>,
    //     object: &Object,
    //     sub: u8,
    //     offset: usize,
    //     buf: &[u8]
    // ) -> Result<(), AbortCode> {
    //     let binding2 = ctx.unwrap().as_any().downcast_ref::<CallbackContext>().unwrap();
    //     binding2.write.fetch_add(1);

    //     Ok(())
    // }

    // let mut od = ObjectDict::new(&OD_TABLE);
    // let ctx = CallbackContext::default();
    // od.register_hook(0x1001, Some(&ctx), Some(read_callback), Some(write_callback));
    // let node_state = manual_node::NodeState::new();


    // let node = Node::new(1, &node_state, od);



    // Arrange

    // Act

    // Assert
    assert!(true);
}
