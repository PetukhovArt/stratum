use zed_extension_api as zed;

struct StratumExtension;

impl zed::Extension for StratumExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: "stratum-lsp".into(),
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(StratumExtension);
