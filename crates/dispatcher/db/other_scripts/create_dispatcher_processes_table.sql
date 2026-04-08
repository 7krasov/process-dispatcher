use process_dispatcher;
drop table if exists dispatcher_processes;
CREATE TABLE dispatcher_processes
(
    uuid       VARBINARY(36)                             NOT NULL,
    source_id  INT UNSIGNED                              NOT NULL,
    state      VARCHAR(15)                               NOT NULL,
    type       TINYINT UNSIGNED                          NOT NULL,
    created_at TIMESTAMP(3) DEFAULT CURRENT_TIMESTAMP(3) NOT NULL,
    PRIMARY KEY (uuid)
);

#TODO: Add indexes

