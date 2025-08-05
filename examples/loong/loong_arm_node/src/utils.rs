use coordinate::rbt_cam_model::RbtCamExtrinsics;
use coordinate::RbtWorldPoint3;
use dora_node_api::Metadata;
use dora_node_api::Parameter;
use openloong_sdk_rust::sdk::Arm;

// 定义一个扩展trait
pub trait MetadataExt {
    fn get(&self, key: &str) -> Option<&Parameter>;
    fn get_string(&self, key: &str, default: &str) -> String;
    fn get_bool(&self, key: &str, default: bool) -> bool;
    fn get_int(&self, key: &str, default: i64) -> i64;
    fn get_float(&self, key: &str, default: f64) -> f64;
    fn get_list_int(&self, key: &str, default: Vec<i64>) -> Vec<i64>;
    fn get_list_float(&self, key: &str, default: Vec<f64>) -> Vec<f64>;
    fn get_list_string(&self, key: &str, default: Vec<String>) -> Vec<String>;
}

// 为Metadata实现这个trait
impl MetadataExt for Metadata {
    fn get(&self, key: &str) -> Option<&Parameter> {
        self.parameters.get(key)
    }

    fn get_string(&self, key: &str, default: &str) -> String {
        if let Some(Parameter::String(value)) = self.parameters.get(key) {
            value.clone()
        } else {
            default.to_string()
        }
    }

    fn get_bool(&self, key: &str, default: bool) -> bool {
        if let Some(Parameter::Bool(value)) = self.parameters.get(key) {
            *value
        } else {
            default
        }
    }

    fn get_int(&self, key: &str, default: i64) -> i64 {
        if let Some(Parameter::Integer(value)) = self.parameters.get(key) {
            *value
        } else {
            default
        }
    }

    fn get_float(&self, key: &str, default: f64) -> f64 {
        match self.parameters.get(key) {
            Some(Parameter::Float(value)) => *value,
            Some(Parameter::Integer(value)) => *value as f64,
            _ => default,
        }
    }

    fn get_list_int(&self, key: &str, default: Vec<i64>) -> Vec<i64> {
        if let Some(Parameter::ListInt(value)) = self.parameters.get(key) {
            value.clone()
        } else {
            default
        }
    }

    fn get_list_float(&self, key: &str, default: Vec<f64>) -> Vec<f64> {
        if let Some(Parameter::ListFloat(value)) = self.parameters.get(key) {
            value.clone()
        } else {
            default
        }
    }

    fn get_list_string(&self, key: &str, default: Vec<String>) -> Vec<String> {
        if let Some(Parameter::ListString(value)) = self.parameters.get(key) {
            value.clone()
        } else {
            default
        }
    }
}

/// 将 gen_72 双臂的坐标转换为 loong arm 的坐标
/// 需要注意之前 gen_72 双臂各自在肩关节处拥有一个自己的坐标系
/// 目前需要统一到 loong arm 的坐标系，原点位于机器人髋关节部位
/// 这里都是直角的变换所以就没有用旋转矩阵，直接赋值了
pub fn coordinate_trans(arm: &Arm, raw: &Vec<f64>) -> Vec<f64> {
    if raw.len() != 6 {
        println!("raw data length is not 6, return empty vector");
        return vec![];
    }
    let raw_x = raw[0];
    let raw_y = raw[1];
    let raw_z = raw[2];
    let [x, y, z] = match arm {
        Arm::Right => [raw_x - 0.4, -raw_z - 0.1, raw_y],
        Arm::Left => [raw_x - 0.25, raw_z + 0.1, -raw_y],
    };
    vec![x, y, z, raw[3], raw[4], raw[5]]
}

/// 直接将相机坐标系中的坐标转换为 loong base 坐标系
pub fn cam_to_base(cam_coord: &[f64; 3]) -> Result<[f32; 3], String> {
    let loong_cam_extrinsics = loong_cam_extrinsics();
    let cam_p = RbtWorldPoint3::new(cam_coord[0], cam_coord[1], cam_coord[2]);
    let base_p: RbtWorldPoint3 = loong_cam_extrinsics
        .isometry()
        .transform_point(&cam_p.point())
        .coords
        .into();
    Ok([base_p.x() as f32, base_p.y() as f32, base_p.z() as f32])
}

fn loong_cam_extrinsics() -> RbtCamExtrinsics {
    // 把相机坐标轴（右-下-前）表示为在机体坐标系（前-左-上）中的方向
    // 注意 Rotation3 是按列定义三个轴（X, Y, Z）
    let cam_axes_to_body_axes_rotation =
        na::Rotation3::from_matrix_unchecked(nalgebra::Matrix3::new(
            0.0, 0.0, 1.0, // X_cam → Z_body
            -1.0, 0.0, 0.0, // Y_cam → -X_body
            0.0, -1.0, 0.0, // Z_cam → -Y_body
        ));

    let pitch_rad = 25_f64.to_radians();
    let pitch_rotation = na::Rotation3::from_euler_angles(0.0, pitch_rad, 0.0);

    // 总旋转：先变换坐标系，再添加俯仰角（注意乘法顺序！先右乘坐标轴转换，再左乘pitch旋转）
    let total_rotation = pitch_rotation * cam_axes_to_body_axes_rotation;

    let translation = na::Translation3::new(-0.035, 0.01, 0.38);
    let cam_extrinsics = RbtCamExtrinsics::new(total_rotation, translation);

    cam_extrinsics
}

// 机械臂运行状态机
#[derive(Debug)]
pub enum LoongArmStage {
    Init,  // 初始化阶段，只会在第一次初始化时使用，后续将会使用预设动作直接完成归位，不再使用此状态
    Grasp, // 抓取阶段，在工作线程中具体包含移动和抓取两个动作
    Release, // 释放阶段，在工作线程中具体包含移动、释放和归位三个动作
}

pub const ACTION_WAIT_SEC: u64 = 8; // 动作等待时间，单位为秒
pub const DEFAULT_MOVE_INCREMENT: f64 = 0.05;
