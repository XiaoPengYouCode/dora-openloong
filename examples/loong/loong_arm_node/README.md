# loong_arm_node

A dora-rs node for basic usage of loong arm. The data frame here [gen72](https://github.com/wangjianpeng200/dora_gen72_test/blob/main/examples/gen72/pick-place-gen72.py)

## how to use

1. add this node to nodehub
1. add this node in your dataflow.yml
1. change the dependency path in Cargo.toml(use `penloong_sdk_basic_usage_example_rust = { git = "https://github.com/XiaoPengYouCode/loong_sim_sdk_release_rust.git" }` instead of local path, or you can clone the loong_sim_sdk_release_rust and add the path the Cargo.toml)
1. run `dora build` and `dora run`
