
create table disk_urls
(
    version_id bigint                                                 not null references versions,
    platform            varchar(100)                                           not null,
    url            varchar(2000)                                           not null
);

alter table versions drop column disk_url;
