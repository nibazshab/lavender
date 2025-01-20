package router

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/nibazshab/webnote/internal/handle"
	"github.com/nibazshab/webnote/internal/log"
	"github.com/nibazshab/webnote/pkg/util"
)

func Router(r *gin.Engine) {
	s := r.Group("/")
	{
		s.GET("/favicon.ico", cacheControl, func(c *gin.Context) {
			c.Data(http.StatusOK, "image/x-icon", []byte{})
		})

		g := s.Group("/assets")
		{
			g.Use(cacheControl)
			handle.StaticAssets(g)
		}

		s.GET("/", redirectNewPath)
		s.GET("/:id", getReqPathId)
		s.POST("/:id", postReqPathId)
	}
}

func getReqPathId(c *gin.Context) {
	id := c.Param("id")

	if urlPathCheck(id) {
		handle.GetDataById(c, &id)
	} else {
		redirectNewPath(c)
	}
}

func postReqPathId(c *gin.Context) {
	id := c.Param("id")

	if urlPathCheck(id) {
		msg, r := handle.PostDataToId(c, &id)
		if r {
			log.Logging(c, id, msg)
		}
	} else {
		c.String(http.StatusBadRequest, "ERROR: invalid path")
	}
}

func redirectNewPath(c *gin.Context) {
	const num = 4
	c.Redirect(http.StatusFound, "/"+util.RandStr(num))
}
