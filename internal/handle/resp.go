package handle

import (
	"html/template"
	"log"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/nibazshab/webnote/web"
)

func webPage(c *gin.Context, id *string, con *string) {
	err := template.Must(template.ParseFS(web.Web, "public/index.html")).Execute(c.Writer, struct {
		URL *string
		CON *string
	}{
		URL: id,
		CON: con,
	})
	if err != nil {
		c.String(http.StatusInternalServerError, "ERROR: internal server error")
		log.Printf("web page error: %v", err)
		return
	}
}

func rawData(c *gin.Context, con *string) {
	c.Data(http.StatusOK, "text/plain; charset=utf-8", []byte(*con))
}
