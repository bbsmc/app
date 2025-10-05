CREATE TABLE version_link_version (
    version_id bigint REFERENCES versions ON UPDATE CASCADE NOT NULL,
    joining_version_id bigint REFERENCES versions ON UPDATE CASCADE NOT NULL,
    PRIMARY KEY (version_id, joining_version_id)
);