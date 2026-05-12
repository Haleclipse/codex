// @cometix: permanently delete a thread — removes DB rows + rollout file.
//
// A rollout JSONL file maps 1:1 to a thread. Multiple session_meta entries
// in the same file represent resume events for the same thread (the first
// session_meta is canonical). Deleting the entire file is safe.

use codex_rollout::find_thread_path_by_id_str;

use super::LocalThreadStore;
use crate::DeleteThreadParams;
use crate::ThreadStoreError;
use crate::ThreadStoreResult;

pub(super) async fn delete_thread(
    store: &LocalThreadStore,
    params: DeleteThreadParams,
) -> ThreadStoreResult<()> {
    let thread_id = params.thread_id;
    let id_str = thread_id.to_string();

    // 1. Delete DB rows (cascades to related tables).
    if let Some(ctx) = codex_rollout::state_db::get_state_db(&store.config).await
        && let Err(err) = ctx.delete_thread_cascade(thread_id).await
    {
        tracing::warn!(
            thread_id = %thread_id,
            error = %err,
            "failed to delete thread rows from state DB"
        );
    }

    // 2. Locate and delete the rollout file (check sessions/ then archived_sessions/).
    let rollout_path =
        match find_thread_path_by_id_str(store.config.codex_home.as_path(), &id_str).await {
            Ok(Some(path)) => Some(path),
            Ok(None) => codex_rollout::find_archived_thread_path_by_id_str(
                store.config.codex_home.as_path(),
                &id_str,
            )
            .await
            .unwrap_or(None),
            Err(err) => {
                return Err(ThreadStoreError::Internal {
                    message: format!("failed to locate thread {thread_id}: {err}"),
                });
            }
        };

    if let Some(path) = rollout_path
        && path.exists()
    {
        std::fs::remove_file(&path).map_err(|err| ThreadStoreError::Internal {
            message: format!("failed to delete rollout file {}: {err}", path.display()),
        })?;
    }

    Ok(())
}
