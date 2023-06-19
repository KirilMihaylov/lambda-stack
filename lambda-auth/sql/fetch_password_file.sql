SELECT "public"."password_files"."setup", "public"."password_files"."password_file"
FROM "public"."password_files"
WHERE "public"."password_files"."user" = $1
  AND "public"."password_files"."expires" IS NOT NULL
  AND CURRENT_TIMESTAMP < "public"."password_files"."expires";

