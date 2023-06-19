INSERT INTO "public"."password_files"("user", "setup", "password_file", "expires")
VALUES ($1, $2, $3, CURRENT_TIMESTAMP + '1day');

