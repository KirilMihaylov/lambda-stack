SELECT "public"."vault"."secret"
FROM "public"."vault"
WHERE "public"."vault"."identifier" = $1
LIMIT 1;

