-- v22 Phase 6: Posts kanban fields.
--
-- Adds fields needed for a professional kanban: sort_order (drag-to-
-- reorder within a column), kanban_substate (ready/in_review/blocked),
-- due_date, priority. These power the kanban re-architecture.

ALTER TABLE posts ADD COLUMN IF NOT EXISTS kanban_sort_order INT NOT NULL DEFAULT 0;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS kanban_substate TEXT
  CHECK (kanban_substate IS NULL OR kanban_substate IN ('ready_to_publish', 'in_review', 'blocked'));
ALTER TABLE posts ADD COLUMN IF NOT EXISTS due_date TIMESTAMPTZ;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS priority TEXT NOT NULL DEFAULT 'medium'
  CHECK (priority IN ('low', 'medium', 'high', 'urgent'));

-- Index for kanban ordering within a state column.
CREATE INDEX IF NOT EXISTS idx_posts_kanban_order
  ON posts(user_id, state, kanban_sort_order)
  WHERE deleted_at IS NULL;

-- Index for due-date queries (overdue cards).
CREATE INDEX IF NOT EXISTS idx_posts_due_date
  ON posts(user_id, due_date)
  WHERE due_date IS NOT NULL AND deleted_at IS NULL;
