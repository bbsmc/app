-- 讨论主题表
create table discussions
(
    id         bigint                                                     not null
    primary key,
    title      varchar(1000)            default ''::character varying     not null,
    content    varchar(65536)           default ''::character varying     not null,
    category   varchar(100)                                               not null,
    created_at timestamp with time zone default CURRENT_TIMESTAMP         not null,
    updated_at timestamp with time zone,
    user_id    bigint                                                     not null references users,
    state      varchar                  default 'open'::character varying not null,
    pinned     boolean                  default false                     not null,
    deleted    boolean                  default false                     not null,
    deleted_at timestamp with time zone
    );


create table posts
(
    id            bigint                                                 not null
        primary key,
    discussion_id bigint                                                 not null references discussions,
    content       varchar(65536)           default ''::character varying not null,
    created_at    timestamp with time zone default CURRENT_TIMESTAMP     not null,
    updated_at    timestamp with time zone default CURRENT_TIMESTAMP,
    user_id       bigint                                                 not null references users,
    replied_to    bigint references posts,
    deleted       boolean                  default false                 not null,
    deleted_at    timestamp with time zone
);



ALTER TABLE mods ADD COLUMN forum bigint REFERENCES discussions(id)  NULL;
