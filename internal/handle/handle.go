package handle

import (
	"io/fs"
	"log"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/nibazshab/webnote/web"
)

func PostDataToId(c *gin.Context, id *string) (rune, bool) {
	if !conTypeCheck(c) {
		c.String(http.StatusBadRequest, "ERROR: content-type not application/x-www-form-urlencoded")
		return 0, false
	}

	const maxBytes = int64(100 << 20)
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, maxBytes)

	con, r := c.GetPostForm("t")
	if r {
		if con == "" {
			deleteDbData(id)
			return 'd', true
		} else {
			writeDbData(id, &con)
			return 'i', true
		}
	} else {
		c.String(http.StatusBadRequest, "ERROR: body not t")
		return 0, false
	}
}

func GetDataById(c *gin.Context, id *string) {
	con := getDbData(id)

	if reqDataTypeCheck(c) {
		rawData(c, con)
	} else {
		webPage(c, id, con)
	}
}

func StaticAssets(g *gin.RouterGroup) {
	public, err := fs.Sub(web.Web, "public/assets")
	if err != nil {
		g.GET("/*file", func(c *gin.Context) {
			c.String(http.StatusInternalServerError, "ERROR: internal server error")
		})
		log.Printf("static assets error: %v", err)
		return
	}

	g.StaticFS("/", http.FS(public))
}
