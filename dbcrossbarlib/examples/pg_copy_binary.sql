DROP TABLE IF EXISTS copy_binary_test;

CREATE TABLE copy_binary_test (
    test_null smallint,
    test_float32 real,
    test_float64 double precision,
    test_i16 smallint,
    test_i32 integer,
    test_i64 bigint
);

\copy copy_binary_test FROM 'pg_copy_binary.bin' WITH BINARY

SELECT * FROM copy_binary_test;

