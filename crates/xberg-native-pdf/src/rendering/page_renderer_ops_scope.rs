//! ExtGState and marked-content operator arms (`gs`, `BDC`) of the operator loop.

use super::*;

impl PageRenderer {
    pub(super) fn op_set_ext_g_state(&mut self, st: &mut OpLoopState<'_>, env: OpEnv<'_>, dict_name: &String) {
        let OpEnv { doc, .. } = env;
        // Fast path: resource dict is already resolved (see top of
        // this function), so the per-`gs` cost is one HashMap
        // lookup + one resolve of the small inner state dict. ~keep
        let entry = st.ext_g_state_cache.entry(dict_name.clone()).or_insert_with(|| {
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
}
