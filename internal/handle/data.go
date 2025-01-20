package handle

import "github.com/nibazshab/webnote/internal/db"

func writeDbData(id *string, con *string) {
	writeData := &db.Data{
		ID:  convHashId(id),
		Con: *con,
	}
	writeData.WriteData()
}

func deleteDbData(id *string) {
	dataDelId := &db.Data{
		ID: convHashId(id),
	}
	dataDelId.DeleteData()
}

func getDbData(id *string) *string {
	dataGetId := &db.Data{
		ID: convHashId(id),
	}
	getData := dataGetId.GetData()
	return &getData.Con
}
