-- -- 先清空 posts 表
TRUNCATE TABLE posts CASCADE;
--
-- -- 清空 discussions 表
-- TRUNCATE TABLE discussions CASCADE;

-- 重新插入讨论主题
INSERT INTO discussions (id, title, category, user_id, state) 
VALUES (1, '欢迎来到项目讨论区！', 'project', 187799526438262, 'open');

-- 插入200个帖子的函数
DO $$
DECLARE
    i INTEGER;
    reply_chance FLOAT;
    random_floor_number INTEGER;
    total_posts INTEGER := 200;
    base_time TIMESTAMP;
    current_timestamp_millis BIGINT;
    random_id BIGINT;  -- 新增变量用于存储随机ID
BEGIN
    -- 初始化基础时间为当前时间减去200天
    base_time := CURRENT_TIMESTAMP - INTERVAL '200 days';

    FOR i IN 1..total_posts LOOP
        -- 生成6-11位随机ID
        random_id := floor(random() * (999999999999 - 100000)) + 100000;

        -- 生成基于时间戳的ID
        current_timestamp_millis := EXTRACT(EPOCH FROM (base_time + (i || ' minutes')::INTERVAL)) * 1000;
        
        -- 随机决定是否回复其他帖子 (30% 的概率)
        reply_chance := random();
        
        -- 如果不是第一个帖子，且随机数小于0.3，则是回复帖子
        IF i > 1 AND reply_chance < 0.3 THEN
            -- 随机选择一个已存在的楼层号作为回复对象
            SELECT floor_number INTO random_floor_number
            FROM posts
            WHERE floor_number < i
            ORDER BY random()
            LIMIT 1;
        ELSE
            random_floor_number := NULL;
        END IF;

        -- 插入帖子
        INSERT INTO posts (
            id,
            discussion_id,
            floor_number,
            content,
            created_at,
            updated_at,
            user_id,
            replied_to
        ) VALUES (
            random_id,  -- 使用随机ID
            1,
            i,
            CASE 
                WHEN random_floor_number IS NULL THEN
                    CASE (random() * 4)::INTEGER
                        WHEN 0 THEN '这个项目太棒了！期待后续的更新。'
                        WHEN 1 THEN '我在使用过程中遇到了一些问题，希望能得到帮助。'
                        WHEN 2 THEN '建议增加一些新功能，比如...'
                        WHEN 3 THEN '感谢分享，这对我帮助很大。'
                        ELSE '这个实现方式很有创意！'
                    END
                ELSE
                    CASE (random() * 4)::INTEGER
                        WHEN 0 THEN '同意楼上的观点！'
                        WHEN 1 THEN '我也遇到类似的情况，一起讨论下。'
                        WHEN 2 THEN '这个问题我可能知道解决方案...'
                        WHEN 3 THEN '补充一下楼上说的内容...'
                        ELSE '感谢分享经验！'
                    END
            END,
            base_time + (i || ' minutes')::INTERVAL,
            base_time + (i || ' minutes')::INTERVAL,
            187799526438262,
            random_floor_number
        );
    END LOOP;

    -- 更新讨论的最后回复时间
    UPDATE discussions 
    SET updated_at = (SELECT MAX(created_at) FROM posts WHERE discussion_id = 1)
    WHERE id = 1;
END $$; 