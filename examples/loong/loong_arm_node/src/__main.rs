use dora_node_api::{DoraNode, Event, arrow::array::StringArray};
use openloong_sdk_rust::sdk::LoongManiSdk;
use std::error::Error;

use openloong_sdk_rust::param::LoongManiParam;

fn main() -> Result<(), Box<dyn Error>> {
    let (_, mut events) = DoraNode::init_from_env()?;
    println!("test println");
    let param = LoongManiParam::read_from_toml().expect("Failed while reading toml file");
    let mut loong_mani_sdk = LoongManiSdk::from_param(&param);

    while let Some(event) = events.recv() {
        match event {
            Event::Input {
                id,
                metadata: _,
                data,
            } => {
                if id.to_string() == "char" {
                    if let Some(input_char) = data.as_any().downcast_ref::<StringArray>() {
                        println!("get input char");
                        let char = input_char.value(0).chars().next().unwrap();
                        print!("char: {char}");
                        parse_input_char(&char, &mut loong_mani_sdk);
                    } else {
                        println!("get input, but not stringArray");
                    }
                    loong_mani_sdk.send().expect("Sdk failed to send message")
                } else {
                    continue;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn parse_input_char(char: &char, sdk: &mut LoongManiSdk) {
    match char {
        'w' => sdk.w("left").unwrap(),
        'a' => sdk.a("left").unwrap(),
        's' => sdk.s("left").unwrap(),
        'd' => sdk.d("left").unwrap(),
        _ => {
            println!("Invalid input")
        }
    }
}
