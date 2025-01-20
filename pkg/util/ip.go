package util

import "net/http"

func GetUserIP(req *http.Request) string {
	ip := req.Header.Get("X-Forwarded-For")
	if ip == "" {
		ip = req.RemoteAddr
	}

	return ip
}
