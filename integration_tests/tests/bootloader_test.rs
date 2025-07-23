mod utils;
use utils::setup_single_node;
use zencan_common::constants::{
    self,
    values::{BOOTLOADER_ERASE_CMD, BOOTLOADER_RESET_CMD},
};

use crate::utils::{test_with_background_process, BusLogger};

use integration_tests::{object_dict2, object_dict3};

const BOOTLOADER_INFO_INDEX: u16 = 0x5500;
const BOOTLOADER_SECTION0_INDEX: u16 = 0x5510;

#[serial_test::serial]
#[tokio::test]
async fn test_device_info_readback() {
    let (mut node, mut client, mut bus) = setup_single_node(
        &object_dict2::OD_TABLE,
        &object_dict2::NODE_MBOX,
        &object_dict2::NODE_STATE,
    );

    let _logger = BusLogger::new(bus.new_receiver());

    let test_task = async move {
        // Highest sub index
        assert_eq!(3, client.read_u8(BOOTLOADER_INFO_INDEX, 0).await.unwrap());
        // Config - application mode, can reset to bootloader
        assert_eq!(3, client.read_u32(BOOTLOADER_INFO_INDEX, 1).await.unwrap());
        // Number of sections
        assert_eq!(1, client.read_u8(BOOTLOADER_INFO_INDEX, 2).await.unwrap());

        assert_eq!(false, object_dict2::BOOTLOADER_INFO.reset_flag());

        client
            .write_u32(BOOTLOADER_INFO_INDEX, 3, BOOTLOADER_RESET_CMD)
            .await
            .unwrap();

        assert_eq!(true, object_dict2::BOOTLOADER_INFO.reset_flag());
    };

    test_with_background_process(&mut [&mut node], &mut bus.new_sender(), test_task).await;
}

#[serial_test::serial]
#[tokio::test]
async fn test_program() {
    let (mut node, mut client, mut bus) = setup_single_node(
        &object_dict3::OD_TABLE,
        &object_dict3::NODE_MBOX,
        &object_dict3::NODE_STATE,
    );

    let _logger = BusLogger::new(bus.new_receiver());

    let test_task = async move {
        // Mode register should indicate this section is programmable
        assert_eq!(
            client.read_u8(BOOTLOADER_SECTION0_INDEX, 1).await.unwrap(),
            1
        );
        // Check the name value
        assert_eq!(
            client
                .read_visible_string(BOOTLOADER_SECTION0_INDEX, 2)
                .await
                .unwrap(),
            "application"
        );
        // Send erase command
        client
            .write_u32(BOOTLOADER_SECTION0_INDEX, 3, BOOTLOADER_ERASE_CMD)
            .await
            .unwrap();
    };

    test_with_background_process(&mut [&mut node], &mut bus.new_sender(), test_task).await;
}
