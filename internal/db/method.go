package db

//type DataMethod interface {
//	WriteData()
//	DeleteData()
//	GetData() *Data
//}

func (data *Data) WriteData() {
	db.Save(data)
}

func (data *Data) DeleteData() {
	db.Where(data).Delete(&Data{})
}

func (data *Data) GetData() *Data {
	db.Where(data).First(data)
	return data
}
