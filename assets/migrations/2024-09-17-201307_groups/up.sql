CREATE TABLE "groups" (
	"id"	INTEGER UNIQUE,
	"title"	TEXT NOT NULL,
	"username"	TEXT UNIQUE,
	"language_code"	TEXT NOT NULL,
	"last_character_id"	INTEGER,
	"last_character_message_id"	INTEGER,
	PRIMARY KEY("id")
);
