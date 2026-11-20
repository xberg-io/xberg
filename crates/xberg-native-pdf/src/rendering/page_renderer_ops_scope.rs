//! Graphics-state and marked-content operator arms (`q`, `Q`, `gs`, `BDC`, `EMC`) of the operator loop.

use super::*;

impl PageRenderer {
    pub(super) fn op_set_ext_g_state(&mut self, st: &mut OpLoopState<'_>, env: OpEnv<'_>, dict_name: &str) {
        let OpEnv { doc, .. } = env;
        // Fast path: resource dict is already resolved (see the top of
        // `execute_operators_with_backdrop`), so the per-`gs` cost is one HashMap
        // lookup + one resolve of the small inner state dict. ~keep
        let entry = st.ext_g_state_cache.entry(dict_name.to_string()).or_insert_with(|| {
            if let Some(states) = st.ext_g_states {
                if let Some(state_obj) = states.get(dict_name) {
                    return parse_ext_g_state_inner(state_obj, doc).unwrap_or_default();
                }
            }
            ParsedExtGState::default()
        });
        entry.apply(st.gs_stack.current_mut());
    }

    pub(super) fn op_begin_marked_content_dict(
        &mut self,
        st: &mut OpLoopState<'_>,
        env: OpEnv<'_>,
        tag: &str,
        properties: &Object,
    ) {
        let OpEnv { doc, resources, .. } = env;
        let mut is_excluded = false;
        // Tag "OC" scopes can hide content even with empty excluded_layers
        // when the OCMD uses /VE /Not or /P /AllOff/AnyOff (the
        // expression evaluates with all OCGs on by default). We can
        // only short-circuit cheaply for simple OCG refs, which the
        // optional_content module handles internally. ~keep
        if tag == "OC" {
            is_excluded = crate::optional_content::resolve_and_check_ocg_excluded(
                properties,
                Some(resources),
                Some(doc),
                st.excluded_layers,
            );
        }
        if is_excluded {
            st.excluded_layer_depth += 1;
        }
        st.marked_content_is_excluded.push(is_excluded);
    }

    pub(super) fn op_save_state(&mut self, st: &mut OpLoopState<'_>) {
        st.gs_stack.save();
        // Clone current clip for the new graphics state level
        // This allows the current level to modify its clip without affecting parents
        // ~keep
        let current_clip = st.clip_stack.last().cloned().flatten();
        st.clip_stack.push(current_clip);
        tracing::trace!(
            "q (SaveState), depth={}, clip_stack depth={}",
            st.gs_stack.depth(),
            st.clip_stack.len()
        );
    }

    pub(super) fn op_restore_state(&mut self, st: &mut OpLoopState<'_>) {
        st.gs_stack.restore();
        if st.clip_stack.len() > 1 {
            st.clip_stack.pop();
        }
        tracing::trace!(
            "Q (RestoreState), depth={}, clip_stack depth={}",
            st.gs_stack.depth(),
            st.clip_stack.len()
        );
    }

    pub(super) fn op_end_marked_content(&mut self, st: &mut OpLoopState<'_>) {
        if let Some(was_excluded) = st.marked_content_is_excluded.pop() {
            if was_excluded && st.excluded_layer_depth > 0 {
                st.excluded_layer_depth -= 1;
            }
        }
    }
}
