use crate::editor_input_context::EditorInputContext;

pub struct EditorUITickContext<'a> {
    pub selected_mode: &'a mut usize,
    pub input_context: EditorInputContext
}