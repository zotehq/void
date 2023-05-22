package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"strconv"
	"strings"
	"sync"
	"time"
)

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

var store = []DataStructure{}
var config Config

type Config struct {
	HTTPPort   string `json:"http_port"`
	TCPPort    string `json:"tcp_port"`
	Username   string `json:"username"`
	Password   string `json:"password"`
	StorePath  string `json:"store_path"`
	ExpireTime int    `json:"expire_time"`
}

func main() {
	loadConfig()
	loadFromDisk()

	var wg sync.WaitGroup
	wg.Add(3)

	http.HandleFunc("/create", createKey)
	http.HandleFunc("/count", count)
	http.HandleFunc("/get", getKey)
	http.HandleFunc("/", getAllKeys)
	http.HandleFunc("/delete", deleteKey)

	ticker := time.Tick(time.Second)

	log.Println("-- TEA DB --")

	go func() {
		log.Println("Writing to Disk...")
		for range ticker {
			saveToDisk(store)
		}
	}()

	go func() {
		log.Printf("Started expiry service (expire time: %d seconds)\n", config.ExpireTime)
		defer wg.Done()
		checkExpired()
	}()

	go func() {
		log.Printf("Started HTTP service on port %s\n", config.HTTPPort)
		defer wg.Done()
		http.ListenAndServe(":"+config.HTTPPort, nil)
	}()

	go func() {
		listener, _ := net.Listen("tcp", ":"+config.TCPPort)
		defer listener.Close()
		log.Printf("Started TCP service on port %s\n", config.TCPPort)
		defer wg.Done()
		for {
			conn, _ := listener.Accept()
			go handleConnection(conn)
		}
	}()

	wg.Wait()
}

func handleConnection(conn net.Conn) {
	defer conn.Close()

	fmt.Fprintf(conn, "Connected to TeaDB \n")
	log.Printf("Client %s connected \n", conn.RemoteAddr())

	reader := bufio.NewReader(conn)

	fmt.Fprintf(conn, "Username: ")
	username, _ := reader.ReadString('\n')
	username = strings.TrimRight(username, "\n")
	fmt.Fprintf(conn, "Password: ")
	password, _ := reader.ReadString('\n')
	password = strings.TrimRight(password, "\n")

	if username != config.Username || password != config.Password {
		fmt.Fprintf(conn, "Invalid username or password\n")
		conn.Close()
		return
	}

	for {
		cmd, err := reader.ReadString('\n')
		if err != nil {
			if err == io.EOF {
				log.Printf("Client %s disconnected", conn.RemoteAddr())
			} else {
				log.Println("Error reading command:", err)
			}
			conn.Close()
			return
		}

		cmd = strings.TrimRight(cmd, "\n")
		fields := strings.Fields(cmd)

		switch fields[0] {
		case "SET":
			if len(fields) < 4 {
				fmt.Fprintf(conn, "Invalid command. Usage: SET key value ttl\n")
				continue
			}

			key := fields[1]
			value := fields[2]
			ttl, err := strconv.Atoi(fields[3])

			if err != nil {
				fmt.Fprintf(conn, "Invalid ttl format: %s", err)
				return
			}

			for _, ds := range store {
				if ds.Key == key {
					fmt.Fprintf(conn, "Key %s already exists\n", key)
					return
				}
			}

			store = append(store, DataStructure{
				Key:   key,
				Value: value,
				TTL:   time.Now().Add(time.Duration(ttl) * time.Second),
			})

			fmt.Fprintf(conn, "Key: %s Value: %s with TTL of %d seconds added to store \n", key, value, ttl)

		case "GET":
			if len(fields) < 2 {
				fmt.Fprintf(conn, "Invalid command. Usage: GET key\n")
				continue
			}

			key := fields[1]

			found := false
			for _, ds := range store {
				if ds.Key == key {
					valueBytes, err := json.Marshal(ds.Value)
					if err != nil {
						fmt.Fprintf(conn, "Error marshalling value: %v\n", err)
						return
					}

					fmt.Fprintf(conn, " %s \n", valueBytes)
					found = true
					break
				}
			}
			if !found {
				fmt.Fprintf(conn, "No key called %s found\n", key)
			}

		case "DELETE":
			if len(fields) < 2 {
				fmt.Fprintf(conn, "Invalid command. Usage: DELETE key\n")
				continue
			}

			key := fields[1]

			var index int
			var found bool
			for i, ds := range store {
				if ds.Key == key {
					index = i
					found = true
					break
				}
			}

			if !found {
				fmt.Fprintf(conn, "Key %s was not found \n", key)
				return
			}

			store = append(store[:index], store[index+1:]...)

			fmt.Fprintf(conn, "Key %s was deleted \n", key)
		case "UPDATE":
			if len(fields) < 3 {
				fmt.Fprintf(conn, "Invalid command. Usage: UPDATE key value\n")
				continue
			}

			key := fields[1]
			value := fields[2]

			if key == "" {
				fmt.Fprintf(conn, "Key was not mentioned\n")
				return
			}

			if value == "" {
				fmt.Fprintf(conn, "Value was not mentioned\n")
				return
			}

			found := false
			for i, ds := range store {
				if ds.Key == key {
					store[i].Value = value
					found = true
					break
				}
			}

			if found {
				fmt.Fprintf(conn, "Key %s was updated with value %s\n", key, value)
			} else {
				fmt.Fprintf(conn, "Key %s was not found\n", key)
			}

		default:
			fmt.Fprintf(conn, "Invalid command: %s\n", cmd)
		}
	}
}

func getKey(w http.ResponseWriter, r *http.Request) {
	if r.Method == "GET" {
		key := r.URL.Query().Get("key")
		if key == "" {
			http.Error(w, "Key not provided", http.StatusBadRequest)
			return
		}

		for _, ds := range store {
			if ds.Key == key {
				valueBytes, err := json.Marshal(ds.Value)
				if err != nil {
					http.Error(w, "Error marshalling value", http.StatusInternalServerError)
					return
				}
				w.Header().Set("Content-Type", "application/json")
				w.Write(valueBytes)
				return
			}
		}

		http.Error(w, "Key not found", http.StatusNotFound)
		return
	}

	http.Error(w, "Invalid method", http.StatusMethodNotAllowed)
}

func createKey(w http.ResponseWriter, r *http.Request) {
	if r.Method == "POST" {
		var reqBody RequestBody
		err := json.NewDecoder(r.Body).Decode(&reqBody)
		if err != nil {
			http.Error(w, "Invalid request body", http.StatusBadRequest)
			return
		}

		key := reqBody.Key
		value := reqBody.Value
		second := reqBody.Second

		if key == "" {
			http.Error(w, "Key not provided", http.StatusBadRequest)
			return
		}

		for _, ds := range store {
			if ds.Key == key {
				http.Error(w, "Key already exists", http.StatusConflict)
				return
			}
		}

		store = append(store, DataStructure{
			Key:   key,
			Value: value,
			TTL:   time.Now().Add(time.Duration(second) * time.Second),
		})

		w.WriteHeader(http.StatusCreated)
	} else {
		http.Error(w, "Invalid method", http.StatusMethodNotAllowed)
	}
}

func deleteKey(w http.ResponseWriter, r *http.Request) {
	if r.Method == "DELETE" {
		key := r.URL.Query().Get("key")
		if key == "" {
			http.Error(w, "Key not provided", http.StatusBadRequest)
			return
		}

		var index int
		var found bool
		for i, ds := range store {
			if ds.Key == key {
				index = i
				found = true
				break
			}
		}

		if !found {
			http.Error(w, "Key not found", http.StatusNotFound)
			return
		}

		store = append(store[:index], store[index+1:]...)

		w.WriteHeader(http.StatusOK)
	} else {
		http.Error(w, "Invalid method", http.StatusMethodNotAllowed)
	}
}

func getAllKeys(w http.ResponseWriter, r *http.Request) {
	if r.Method == "GET" {
		keys := make([]string, len(store))
		for i, ds := range store {
			keys[i] = ds.Key
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(keys)
	} else {
		http.Error(w, "Invalid method", http.StatusMethodNotAllowed)
	}
}

func count(w http.ResponseWriter, r *http.Request) {
	if r.Method == "GET" {
		w.Header().Set("Content-Type", "text/plain")
		fmt.Fprintf(w, "%d\n", len(store))
	} else {
		http.Error(w, "Invalid method", http.StatusMethodNotAllowed)
	}
}

func checkExpired() {
	for {
		now := time.Now()

		var expiredKeys []string

		for _, ds := range store {
			if ds.TTL.Before(now) {
				expiredKeys = append(expiredKeys, ds.Key)
			}
		}

		if len(expiredKeys) > 0 {
			for _, key := range expiredKeys {
				for i, ds := range store {
					if ds.Key == key {
						store = append(store[:i], store[i+1:]...)
						break
					}
				}
			}
			log.Printf("Expired keys removed: %s\n", strings.Join(expiredKeys, ", "))
		}

		time.Sleep(time.Second)
	}
}

func loadConfig() {
	_, err := os.Stat("config.json")
	if os.IsNotExist(err) {
		createDefaultConfig()
	}

	file, err := os.Open("config.json")
	if err != nil {
		log.Fatal("Error opening config file:", err)
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	err = decoder.Decode(&config)
	if err != nil {
		log.Fatal("Error decoding config file:", err)
	}
}

func createDefaultConfig() {
	config = Config{
		StorePath: "store.json",
	}

	file, err := os.Create("config.json")
	if err != nil {
		log.Fatal("Error creating config file:", err)
	}
	defer file.Close()

	encoder := json.NewEncoder(file)
	err = encoder.Encode(config)
	if err != nil {
		log.Fatal("Error encoding config:", err)
	}

	log.Println("Default config file created: config.json")
}

func loadFromDisk() {
	file, err := os.Open(config.StorePath)
	if err != nil {
		log.Println("No previous data found")
		return
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	err = decoder.Decode(&store)
	if err != nil {
		log.Fatal("Error decoding data file:", err)
	}
}

func saveToDisk(data []DataStructure) {
	file, err := os.Create(config.StorePath)
	if err != nil {
		log.Fatal("Error creating data file:", err)
	}
	defer file.Close()

	encoder := json.NewEncoder(file)
	err = encoder.Encode(data)
	if err != nil {
		log.Fatal("Error encoding data:", err)
	}
}
