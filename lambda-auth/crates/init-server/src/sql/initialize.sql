CREATE TABLE "public"."password_files" (
    "user"          VARCHAR(127) NOT NULL,
    "setup"         bytea        NOT NULL,
    "password_file" bytea        NOT NULL,
    "expires"       timestamptz  NULL,
    CONSTRAINT "password_files_pkey"
        PRIMARY KEY ("user"),
    CONSTRAINT "non_empty_user_check"
        CHECK ( LENGTH("public"."password_files"."user") != 0 ),
    CONSTRAINT "non_empty_setup_check"
        CHECK ( LENGTH("public"."password_files"."setup") != 0 ),
    CONSTRAINT "non_empty_password_file_check"
        CHECK ( LENGTH("public"."password_files"."password_file") != 0 )
);

