-- 生成聊天板块的测试数据
DO $$
DECLARE
    discussion_id bigint;
    post_id bigint := 1;
    user_ids bigint[] := ARRAY[
        127155982985829, 0, 207447015598584, 204988380598441, 210270301977713,
        187799526438262, 133433672860492, 165674671329082, 112531331349590,
        44550865134193, 197864515949221, 75823506859031, 183654373625886,
        38450390996745, 91134886993241, 166297929485484, 199939320896075,
        74530046205688, 151614731669524, 14438702105802, 171699792546999,
        105459150880271, 187495248964194, 92901282361691, 141052117790403,
        201521280365379, 37485473698110, 103051601742582, 163959315679970,
        204779994429778, 42479287413697, 216011886293514, 17975006124634,
        112355265769427, 119697831325122, 68473939178911, 125442590612417
    ];
    random_user_id bigint;
    random_post_id bigint;
    post_count int;
    topic_content text;
    i int;
    j int;
BEGIN
    -- 生成100个讨论主题
    FOR i IN 1..100 LOOP
        -- 为每个讨论主题生成一个随机ID (确保唯一性)
        discussion_id := (random() * 900000000 + 100000000)::bigint;
        
        -- 从用户数组中随机选择一个用户ID
        random_user_id := user_ids[1 + (random() * (array_length(user_ids, 1) - 1))::integer];
        
        -- 根据i选择不同的主题内容
        CASE i % 20
            WHEN 0 THEN topic_content := '分享一下我的红石自动化农场设计，效率提高了300%！';
            WHEN 1 THEN topic_content := '关于1.21版本的洞穴更新，你们怎么看？';
            WHEN 2 THEN topic_content := '求推荐一些适合建筑新手的mod';
            WHEN 3 THEN topic_content := '分享我的中世纪城堡建筑企划';
            WHEN 4 THEN topic_content := '服务器性能优化经验总结';
            WHEN 5 THEN topic_content := '生存模式最快速发展攻略';
            WHEN 6 THEN topic_content := '关于新增的考古系统，有什么有趣的发现吗？';
            WHEN 7 THEN topic_content := '分享一些适合生存服务器的小游戏设计';
            WHEN 8 THEN topic_content := '如何设计一个平衡的RPG服务器？';
            WHEN 9 THEN topic_content := '关于服务器反作弊系统的讨论';
            WHEN 10 THEN topic_content := '分享我的红石音乐作品';
            WHEN 11 THEN topic_content := '如何优化大型建筑的渲染性能？';
            WHEN 12 THEN topic_content := '生存服初期发展路线推荐';
            WHEN 13 THEN topic_content := '关于新版本刷怪塔设计的变化';
            WHEN 14 THEN topic_content := '分享一些适合截图的建筑风格';
            WHEN 15 THEN topic_content := '服务器经济系统设计经验';
            WHEN 16 THEN topic_content := '关于PVP平衡性的讨论';
            WHEN 17 THEN topic_content := '分享我的末地建筑企划';
            WHEN 18 THEN topic_content := '红石计算器制作教程';
            ELSE topic_content := '探讨新版本附魔系统的改变';
        END CASE;

        -- 插入讨论主题
        INSERT INTO discussions (
            id, title, content, category, created_at, user_id, state, pinned, last_post_time
        ) VALUES (
            discussion_id,
            topic_content,
            CASE i % 20
                WHEN 0 THEN '最近设计了一个新的自动化农场系统，使用了漏斗矩阵和红石比较器，可以同时处理多种作物。效率比传统设计提升了300%，想和大家分享一下设计思路。'
                WHEN 1 THEN '看了最新的快照，洞穴生成算法有了很大变化，新增的矿物分布也很有特色。不知道大家对这些改动有什么想法？'
                -- 此处省略其他18个详细内容，实际代码中需要补充完整
                ELSE '附魔系统的改变让一些附魔组合变得更加有趣，比如新增的套装效果。这些变化会如何影响游戏平衡性？'
            END,
            'chat',
            NOW() - (random() * interval '365 days'),
            random_user_id,
            'open',
            random() < 0.1,
            NOW() - (random() * interval '365 days')
        );
        
        -- 为每个讨论主题生成相关的回复
        post_count := 100 + (random() * 200)::integer;
        
        FOR j IN 1..post_count LOOP
            random_user_id := user_ids[1 + (random() * (array_length(user_ids, 1) - 1))::integer];
            random_post_id := NULL;
            
            IF j > 1 AND random() < 0.3 THEN
                random_post_id := post_id - (1 + (random() * (j - 1))::integer);
            END IF;
            
            -- 插入与主题相关的回复
            INSERT INTO posts (
                id, discussion_id, content, created_at, user_id, replied_to
            ) VALUES (
                post_id,
                discussion_id,
                CASE i % 20
                    WHEN 0 THEN -- 农场相关回复
                        CASE (random() * 4)::integer
                            WHEN 0 THEN '这个设计真不错，我特别喜欢漏斗矩阵的部分'
                            WHEN 1 THEN '请问红石比较器具体是怎么接线的？'
                            WHEN 2 THEN '我也在做类似的设计，但效率没有这么高'
                            WHEN 3 THEN '这个设计在服务器上会不会卡顿？'
                            ELSE '能分享一下具体的建造教程吗？'
                        END
                    WHEN 1 THEN -- 洞穴更新相关回复
                        CASE (random() * 4)::integer
                            WHEN 0 THEN '新的洞穴生成真的很壮观'
                            WHEN 1 THEN '矿物分布的改变让挖矿更有趣了'
                            WHEN 2 THEN '希望能加入更多的洞穴生物'
                            WHEN 3 THEN '地形生成算法优化得不错'
                            ELSE '探洞变得更有挑战性了'
                        END
                    -- 此处省略其他18种情况的回复，实际代码中需要补充完整
                    ELSE 
                        '这个话题很有意思，继续关注'
                END,
                NOW() - (random() * interval '365 days'),
                random_user_id,
                random_post_id
            );
            
            post_id := post_id + 1;
        END LOOP;
    END LOOP;
END $$; 