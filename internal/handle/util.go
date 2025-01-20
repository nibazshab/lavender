package handle

import (
	"hash/fnv"
	"regexp"
	"strings"

	"github.com/gin-gonic/gin"

	"github.com/nibazshab/webnote/pkg/util"
)

func conTypeCheck(c *gin.Context) bool {
	return strings.HasPrefix(c.Request.Header.Get("Content-Type"), "application/x-www-form-urlencoded")
}

var regexUa = regexp.MustCompile(`^(curl|Wget)`)

func reqDataTypeCheck(c *gin.Context) bool {
	return regexUa.MatchString(util.GetUserUA(c.Request)) || c.Request.URL.Query().Has("raw")
}

func convHashId(s *string) uint32 {
	h := fnv.New32a()
	_, _ = h.Write([]byte(*s))
	return h.Sum32()
}
