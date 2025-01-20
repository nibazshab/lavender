package util

import "net/http"

func GetUserUA(req *http.Request) string {
	return req.Header.Get("user-agent")
}
