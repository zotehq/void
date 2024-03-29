package structs

import "time"

type DataStructure struct {
	Key   string
	Value string
	TTL   time.Time
}

type RequestBody struct {
	Key    string `json:"key"`
	Value  string `json:"value"`
	Second int    `json:"second"`
}

type Config struct {
	HTTPPort   string `json:"http_port"`
	TCPPort    string `json:"tcp_port"`
	Username   string `json:"username"`
	Password   string `json:"password"`
	StorePath  string `json:"store_path"`
	ExpireTime int    `json:"expire_time"`
}
