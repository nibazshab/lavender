package router

import (
	"regexp"

	"github.com/gin-gonic/gin"
)

var regexPath = regexp.MustCompile(`^[a-zA-Z0-9]+$`)

func urlPathCheck(id string) bool {
	return regexPath.MatchString(id) && len(id) < 17
}

func cacheControl(c *gin.Context) {
	c.Header("Cache-Control", "public, max-age=3600")
	c.Next()
}
