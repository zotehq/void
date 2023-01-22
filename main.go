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

func main() {
	var wg sync.WaitGroup
	wg.Add(3)

	http.HandleFunc("/create", createKey)
	http.HandleFunc("/count", count)
	http.HandleFunc("/get", getKey)
	http.HandleFunc("/", getAllKeys)
	http.HandleFunc("/delete", deleteKey)

	ticker := time.Tick(time.Second)

	log.Println("-- TEA DB --")

	loadFromDisk()

	go func() {
		log.Println("Writing to Disk...")
		for range ticker {
			saveToDisk(store)
		}
	}()

	go func() {
		log.Println("Started expiry service")
		defer wg.Done()
		checkExpired()
	}()

	go func() {
		log.Println("Started HTTP service on port 8080")
		defer wg.Done()
		http.ListenAndServe("0.0.0.0:8080", nil)
	}()

	go func() {
		listener, _ := net.Listen("tcp", ":9000")
		defer listener.Close()
		log.Println("Started TCP listening on port 9000")
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

	// fmt.Fprintf(conn, "Username: ")
	// username, _ := reader.ReadString('\n')
	// username = strings.TrimRight(username, "\n")
	// fmt.Fprintf(conn, "Password: ")
	// password, _ := reader.ReadString('\n')
	// password = strings.TrimRight(password, "\n")

	// if username != "tea" || password != "tea" {
	// 	fmt.Fprintf(conn, "Invalid username or password\n")
	// 	conn.Close()
	// 	return
	// }

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

		if fields[0] == "SET" {
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
		} else if fields[0] == "GET" {
			key := fields[1]

			for _, ds := range store {
				if ds.Key == key {
					valueBytes, err := json.Marshal(ds.Value)
					if err != nil {
						fmt.Fprintf(conn, "No key called %s found\n", key)

						return
					}

					fmt.Fprintf(conn, " %s \n", valueBytes)

				}
			}

		} else {
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
	} else {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
	}
}

func getAllKeys(w http.ResponseWriter, r *http.Request) {
	if r.Method == "GET" {
		dataBytes, err := json.Marshal(store)
		if err != nil {
			http.Error(w, "Error marshalling store", http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.Write(dataBytes)
	} else {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
	}
}

func deleteKey(w http.ResponseWriter, r *http.Request) {
	if r.Method == "DELETE" {
		key := r.URL.Query().Get("key")

		if key == "" {
			http.Error(w, "Key parameter is missing", http.StatusBadRequest)
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
			http.Error(w, "Key was not found", http.StatusNotFound)
			return
		}

		store = append(store[:index], store[index+1:]...)

		w.Write([]byte("Key was deleted successfully"))
	} else {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
	}

}

func count(w http.ResponseWriter, r *http.Request) {
	if r.Method == "GET" {
		count := len(store)
		countBytes, err := json.Marshal(count)
		if err != nil {
			http.Error(w, "Error marshalling count", http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write(countBytes)
	} else {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
	}
}

func createKey(w http.ResponseWriter, r *http.Request) {
	if r.Method == "POST" {
		body, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w, "Error reading request body",
				http.StatusInternalServerError)
		}

		var data RequestBody
		err = json.Unmarshal(body, &data)

		if err != nil {
			http.Error(w, "Error Unmarshal req body",
				http.StatusInternalServerError)
		}

		for _, ds := range store {
			if ds.Key == data.Key {
				http.Error(w, "Key already exists", http.StatusBadRequest)
				return
			}
		}

		store = append(store, DataStructure{
			Key:   data.Key,
			Value: data.Value,
			TTL:   time.Now().Add(time.Second * time.Duration(data.Second)),
		})

		response := map[string]string{"created": "true", "key": data.Key}

		dataBytes, err := json.Marshal(response)

		if err != nil {
			http.Error(w, "Error marshalling response",
				http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")

		w.Write(dataBytes)

	} else {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
	}

}

func checkExpired() {
	for {
		time.Sleep(time.Second)
		for i := 0; i < len(store); i++ {
			if time.Now().After(store[i].TTL) {
				fmt.Println("Key", store[i].Key+" has expired.")
				store = append(store[:i], store[i+1:]...)
			}
		}
	}
}

func saveToDisk(store []DataStructure) {
	dataBytes, err := json.Marshal(store)
	if err != nil {
		log.Fatalln("Error marshalling store:", err)
		return
	}

	err = os.WriteFile("store.json", dataBytes, 0644)
	if err != nil {
		log.Fatalln("Error writing store to disk:", err)
	}
}

func loadFromDisk() {
	if _, err := os.Stat("store.json"); os.IsNotExist(err) {
		file, err := os.Create("store.json")
		if err != nil {
			log.Fatalln("Error creating store.json:", err)
		}
		file.Close()
	}

	dataBytes, err := os.ReadFile("store.json")
	log.Println("Loading data from store.json")
	if err != nil {
		log.Fatalln("Error reading store from disk:", err)
		return
	}

	err = json.Unmarshal(dataBytes, &store)
	if err != nil {
		log.Fatalln("Error unmarshalling store data:", err)
	}
}
