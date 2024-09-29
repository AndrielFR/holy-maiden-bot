CREATE TABLE "characters" (
	"id"	INTEGER UNIQUE,
	"name"	TEXT NOT NULL,
	"image"	TEXT,
	"stars"	INTEGER NOT NULL DEFAULT 1,
	"available"	INTEGER NOT NULL DEFAULT 1,
	PRIMARY KEY("id")
);
