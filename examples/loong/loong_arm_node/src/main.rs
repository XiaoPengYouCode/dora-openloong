use dora_node_api::{
    arrow::array::Float64Array, dora_core::config::DataId, DoraNode, Event, IntoArrow,
    MetadataParameters,
};

extern crate nalgebra as na;
pub mod state_machine;
pub mod utils;

use openloong_sdk_rust::sdk::Arm;
use state_machine::state_machine;
use utils::{LoongArmStage, MetadataExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut node, mut events) = DoraNode::init_from_env()?;

    // channel for communication between threads
    let (tx, rx) = std::sync::mpsc::channel::<(LoongArmStage, Arm, [f32; 3])>();

    let state_machine_handler = state_machine(rx);

    // init state
    let mut first_init_state = false;

    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, metadata, data } => {
                if id.to_string() == "pose_r" || id.to_string() == "pose_l" {
                    // 以下的get方法均为自定义的对 MetaData 的扩展方法，在 utils.rs 中定义
                    // 因为 Dora 目前没有提供比较好的解包方法
                    // 已经提了 issue
                    let encoding = metadata.get_string("encoding", "xyzrpy");
                    let stage = metadata.get_string("stage", "object");
                    let arm = metadata.get_string("arm", "right");
                    let wait = metadata.get_bool("bool", true);
                    // println!("data: {:?}", data);
                    let values = data
                        .as_any()
                        .downcast_ref::<Float64Array>()
                        .unwrap()
                        .values()
                        .to_vec();

                    if wait {
                        if encoding == "xyzrpy" {
                            if arm == "right" || arm == "left" {
                                let arm = Arm::from_str(arm.as_str()).expect("Failed to parse arm");
                                let loong_arm_stage = if stage == "object" {
                                    LoongArmStage::Grasp
                                } else if stage == "destination" {
                                    LoongArmStage::Release
                                } else {
                                    panic!("Unsupported stage: {}", stage);
                                };

                                // 坐标转换
                                let cmd = [values[0], values[1], values[2]];
                                let long_arm_xyz = utils::cam_to_base(&cmd).unwrap();

                                // send to work thread
                                // 使用 channel 发送数据到工作线程
                                // 避免阻塞节点主线程
                                tx.send((loong_arm_stage, arm, long_arm_xyz))
                                    .expect("Failed to send arm data");

                                // response
                                // 上层节点需要该节点进行响应
                                // 但是查看源码发现没有用到响应的内容，所以随意发一个 "OK" 字符串
                                let parameter = MetadataParameters::default();
                                let outputid = match arm {
                                    Arm::Right => DataId::from("response_r_arm".to_string()),
                                    Arm::Left => DataId::from("response_l_arm".to_string()),
                                };
                                node.send_output(
                                    outputid,
                                    parameter,
                                    String::from("ok").into_arrow(),
                                )?;
                            } else {
                                print!("Unsupported arm: {}", arm);
                            }
                        } else if encoding == "jointstate" && !first_init_state {
                            // 该分支代码只会在第一次初始化时使用
                            // 之后将会使用预设动作直接完成归位，不再使用此状态
                            println!("init loong arm");
                            if arm == "right" || arm == "left" {
                                let outputid = if arm == "right" {
                                    DataId::from("response_r_arm".to_string())
                                } else {
                                    DataId::from("response_l_arm".to_string())
                                };
                                tx.send((LoongArmStage::Init, Arm::Left, [0.0; 3])) // 这里的arm不会用到，随便写了一个
                                    .expect("Failed to send jointstate data");
                                let parameter = MetadataParameters::default();
                                node.send_output(
                                    outputid,
                                    parameter,
                                    String::from("ok").into_arrow(),
                                )?;
                            } else {
                                println!("Unsupported arm: {}", arm);
                            }
                            first_init_state = true;
                        }
                    } else {
                        println!("waiting!");
                    }
                } else {
                    continue;
                }
            }
            _ => {}
        }
    }

    state_machine_handler.join().unwrap();

    Ok(())
}
