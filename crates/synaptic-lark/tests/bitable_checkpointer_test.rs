#[cfg(feature = "checkpointer")]
mod tests {
    use synaptic_lark::{LarkBitableCheckpointer, LarkConfig};

    #[test]
    fn constructor() {
        let cp =
            LarkBitableCheckpointer::new(LarkConfig::new("cli", "secret"), "bascnXxx", "tblXxx");
        assert_eq!(cp.app_token(), "bascnXxx");
    }
}
