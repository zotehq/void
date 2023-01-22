package main

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
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

	wg.Add(2)

	http.HandleFunc("/create", createKey)
	http.HandleFunc("/count", count)
	http.HandleFunc("/get", getAllKeys)

	ticker := time.Tick(time.Second)

	log.Println("-- TEA DB STARTING --")

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

	wg.Wait()
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
	dataBytes, err := os.ReadFile("store.json")
	log.Println("Loading data from store.json")
	if err != nil {
		log.Fatalln("Error reading store from disk:", err)
		return
	}

	err = json.Unmarshal(dataBytes, &store)
	if err != nil {
		fmt.Println("Error unmarshalling store data:", err)
	}
}
