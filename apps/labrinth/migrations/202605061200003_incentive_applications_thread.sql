-- 申请关联 thread（用于作者与版主来回沟通）
ALTER TABLE incentive_applications
    ADD COLUMN thread_id bigint REFERENCES threads ON UPDATE CASCADE NULL;

CREATE INDEX idx_incentive_appl_thread ON incentive_applications (thread_id);
