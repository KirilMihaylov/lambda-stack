SELECT TRUE AS "exists"
WHERE EXISTS (SELECT NULL
              FROM "public"."password_files"
              WHERE "public"."password_files"."user" = $1)
UNION
SELECT FALSE AS "exists"
LIMIT 1;
