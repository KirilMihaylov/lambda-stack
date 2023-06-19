CREATE TABLE "public"."vault" (
    "identifier" VARCHAR(255) NOT NULL,
    "secret"     bytea        NOT NULL,
    CONSTRAINT "vault_pkey"
        PRIMARY KEY ("identifier"),
    CONSTRAINT "identifier_length_check"
        CHECK ( LENGTH("public"."vault"."identifier") != 0 ),
    CONSTRAINT "secret_length_check"
        CHECK ( LENGTH("public"."vault"."secret") != 0 )
);

