CREATE TABLE "characters" (
	"id"	INTEGER UNIQUE,
	"name"	TEXT NOT NULL,
	"image"	TEXT,
	"stars"	INTEGER NOT NULL DEFAULT 1,
	"gender"	TEXT NOT NULL,
	"artist"	TEXT NOT NULL DEFAULT 'Artist',
	"aliases"	TEXT NOT NULL DEFAULT '[]',
	"liked_by"	TEXT NOT NULL DEFAULT '[]',
	"series_id"	INTEGER NOT NULL DEFAULT 0,
	"image_link"	TEXT NOT NULL DEFAULT '.',
	"anilist_id"	INTEGER,
	PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE "groups" (
	"id"	INTEGER UNIQUE,
	"title"	TEXT NOT NULL,
	"username"	TEXT UNIQUE,
	"language_code"	TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE TABLE "group_characters" (
	"id"	INTEGER UNIQUE,
	"group_id"	INTEGER NOT NULL,
	"character_id"	INTEGER NOT NULL,
	"last_message_id"	INTEGER NOT NULL,
	"available"	INTEGER NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE "series" (
	"id"	INTEGER NOT NULL,
	"title"	TEXT NOT NULL,
	"artist"	TEXT NOT NULL DEFAULT 'Artist',
	"banner"	TEXT,
	"aliases"	TEXT NOT NULL DEFAULT '[]',
	"liked_by"	TEXT NOT NULL DEFAULT '[]',
	"image_link"	TEXT NOT NULL DEFAULT '.',
	"media_type"	TEXT NOT NULL DEFAULT 'unknown',
	PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE "users" (
	"id"	INTEGER UNIQUE,
	"username"	TEXT UNIQUE,
	"full_name"	TEXT NOT NULL,
	"language_code"	TEXT NOT NULL,
	PRIMARY KEY("id")
);

CREATE TABLE "user_characters" (
	"id"	INTEGER UNIQUE,
	"user_id"	INTEGER NOT NULL,
	"group_id"	INTEGER NOT NULL,
	"characters_id"	TEXT NOT NULL DEFAULT '[]',
	PRIMARY KEY("id" AUTOINCREMENT)
);
