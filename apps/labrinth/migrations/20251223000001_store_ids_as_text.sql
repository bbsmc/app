-- Modrinth 上游提交 79c263301: Fix slug/project ID collisions (#4844)
-- 添加 text_id 和 text_id_lower 生成列，用于检测 slug 与项目 ID 的冲突

-- 复制现有的 from/to_base62 函数，但使用 IMMUTABLE 以便在生成列中使用

CREATE OR REPLACE FUNCTION from_base62(input VARCHAR)
RETURNS BIGINT AS $$
DECLARE
    base INT := 62;
    chars VARCHAR := '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
    result BIGINT := 0;
    i INT;
    char VARCHAR;
    index INT;
BEGIN
    FOR i IN 1..LENGTH(input) LOOP
        char := SUBSTRING(input FROM i FOR 1);
        index := POSITION(char IN chars) - 1;
        IF index < 0 THEN
            RAISE EXCEPTION 'Error: Invalid character in input string';
        END IF;
        result := result * base + index;
    END LOOP;

    RETURN result;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION to_base62(input BIGINT)
RETURNS VARCHAR AS $$
DECLARE
    base INT := 62;
    chars VARCHAR := '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
    result VARCHAR := '';
    remainder INT;
BEGIN
    WHILE input > 0 LOOP
        remainder := input % base;
        result := SUBSTRING(chars FROM remainder + 1 FOR 1) || result;
        input := input / base;
    END LOOP;

    RETURN result;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- 为 mods 表添加生成列
ALTER TABLE mods
    ADD COLUMN text_id TEXT GENERATED ALWAYS AS (to_base62(id)) STORED,
    ADD COLUMN text_id_lower TEXT GENERATED ALWAYS AS (lower(to_base62(id))) STORED;

-- 为 organizations 表添加生成列
ALTER TABLE organizations
    ADD COLUMN text_id TEXT GENERATED ALWAYS AS (to_base62(id)) STORED,
    ADD COLUMN text_id_lower TEXT GENERATED ALWAYS AS (lower(to_base62(id))) STORED;
