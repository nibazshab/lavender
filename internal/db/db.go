package db

import (
	"log"

	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"
	"gorm.io/gorm/schema"

	"github.com/nibazshab/webnote/internal/path"
)

const (
	dbFileName  = "database.sqlite"
	tablePrefix = "webnote_"
)

type Data struct {
	ID  uint32
	Con string
}

var db *gorm.DB

func Init() {
	var err error
	dbFile := path.GetFilePath(dbFileName)

	db, err = gorm.Open(sqlite.Open(dbFile+"?_journal=WAL&_vacuum=incremental"), &gorm.Config{
		NamingStrategy: schema.NamingStrategy{
			TablePrefix: tablePrefix,
		},
		Logger: logger.Default.LogMode(logger.Silent),
	})
	if err != nil {
		log.Fatalf("db connect error: %v", err)
	}

	err = db.AutoMigrate(&Data{})
	if err != nil {
		log.Fatalf("db init error: %v", err)
	}
}

func Close() error {
	_db, err := db.DB()
	if err != nil {
		return err
	}

	err = _db.Close()
	if err != nil {
		return err
	}

	return nil
}
