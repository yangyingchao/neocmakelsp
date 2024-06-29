use super::Location;
use async_lsp::lsp_types;
use async_lsp::ClientSocket;
use lsp_types::Url;
use std::path::PathBuf;
pub(super) async fn cmpsubdirectory(
    localpath: String,
    subpath: &str,
    _client: &ClientSocket,
) -> Option<Vec<Location>> {
    let path = PathBuf::from(localpath);
    let dir = path.parent().unwrap();
    let target = dir.join(subpath).join("CMakeLists.txt");
    if target.exists() {
        Some(vec![Location {
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 0,
                    character: 0,
                },
            },
            uri: Url::from_file_path(target).unwrap(),
        }])
    } else {
        // client
        //     .log_message(MessageType::INFO, "path not exist")
        //     .await;
        None
    }
}
