// UI state only. Persistence belongs to the existing Store.save/read boundary.
export function createEditor(port) {
  let state = { id: null, revision: 0, text: '', dirty: false, saving: false, error: null };
  let editSequence = 0;
  let loadSequence = 0;
  return {
    snapshot: () => ({ ...state }),
    edit(text) {
      editSequence += 1;
      state = { ...state, text, dirty: true };
    },
    async load(id, revision) {
      if (state.dirty || state.saving) throw new Error('Unsaved changes');
      const sequence = editSequence;
      const request = ++loadSequence;
      const text = await port.read(id, revision);
      if (sequence !== editSequence || request !== loadSequence) return;
      state = { id, revision, text, dirty: false, saving: false, error: null };
    },
    async save() {
      if (state.saving || !state.id || !state.dirty) return;
      const sequence = editSequence;
      const { id, revision, text } = state;
      state = { ...state, saving: true, error: null };
      try {
        const next = await port.save(id, revision, text);
        state = { ...state, revision: next, dirty: sequence !== editSequence };
      } catch (error) {
        state = { ...state, error: error.message };
      } finally {
        state = { ...state, saving: false };
      }
    },
  };
}
