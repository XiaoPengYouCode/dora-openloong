use openloong_sdk_rust::app::basic_motion::GripperAction;
use openloong_sdk_rust::param::LoongManiParam;
use openloong_sdk_rust::sdk::{Arm, LoongManiSdk};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

use crate::utils::{LoongArmStage, ACTION_WAIT_SEC};

/// work thread(工作线程，状态机)
pub fn state_machine(rx: Receiver<(LoongArmStage, Arm, [f32; 3])>) -> thread::JoinHandle<()> {
    // init loong mani sdk
    let param = LoongManiParam::read_from_toml().expect("Failed while reading toml file");
    let mut loong_mani_sdk = LoongManiSdk::from_param(&param);

    thread::spawn(move || {
        let left_init_cmd = [0.1, 0.35, 0.1];
        let right_init_cmd = [0.1, -0.35, 0.1];
        let table_z = param.pick_place_param().table_z();
        while let Ok((stage, arm, mut coord)) = rx.recv() {
            match stage {
                // 包含移动，抓取两个动作
                LoongArmStage::Grasp => {
                    // 将高度修改为桌子的高度，提高稳定性
                    coord[2] = table_z;
                    coord[1] *= 0.75;
                    println!("Grasp stage coordinate: {:?}", coord);

                    // 抓取时候先移动至物体后方，然后再往前进行抓取
                    let mut behind_grab_coord = coord;
                    behind_grab_coord[0] -= 0.13;

                    // 抓取之后
                    let mut after_grab_coord = coord;
                    after_grab_coord[2] += 0.2;

                    loong_mani_sdk
                        .smooth_xyz(&arm, &behind_grab_coord, Duration::from_secs_f32(2.5)) // 移动至物体后方
                        .unwrap()
                        .smooth_xyz(&arm, &coord, Duration::from_secs_f32(2.5)) // 移动至物体位置
                        .unwrap()
                        .handle_gripper(&arm, GripperAction::Grasp, 3) // 执行抓取
                        .unwrap()
                        .smooth_xyz(&arm, &after_grab_coord, Duration::from_secs_f32(2.5)) // 抬高Z轴
                        .unwrap();
                }
                // 包含移动，释放，归位三个动作
                LoongArmStage::Release => {
                    println!("Release stage coordinate: {:?}", coord);

                    let mut release_coord = coord;
                    release_coord[2] += 0.3; // 移动至目标的上方，目标是盒子，一般比较低，所以z轴会多加一点

                    loong_mani_sdk
                        .smooth_xyz(
                            &arm,
                            &release_coord,
                            std::time::Duration::from_secs_f32(4.0),
                        )
                        .unwrap()
                        .handle_gripper(&arm, GripperAction::Release, 3)
                        .unwrap();
                    match arm {
                        Arm::Left => {
                            loong_mani_sdk
                                .smooth_xyz(
                                    &Arm::Left,
                                    &left_init_cmd,
                                    std::time::Duration::from_secs_f32(3.0),
                                )
                                .unwrap();
                        }
                        Arm::Right => {
                            loong_mani_sdk
                                .smooth_xyz(
                                    &Arm::Right,
                                    &right_init_cmd,
                                    std::time::Duration::from_secs_f32(3.0),
                                )
                                .unwrap();
                        }
                    };
                }
                // 包含初始化动作
                LoongArmStage::Init => {
                    println!("Init stage: ");
                    loong_mani_sdk.handle_init(ACTION_WAIT_SEC).unwrap();
                }
            }
        }
    })
}
