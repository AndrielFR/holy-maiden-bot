CREATE TABLE "users" (
	"id"	INTEGER UNIQUE,
	"username"	TEXT UNIQUE,
	"full_name"	TEXT NOT NULL,
	"language_code"	TEXT NOT NULL,
	"owned_characters"	TEXT,
	PRIMARY KEY("id")
);
