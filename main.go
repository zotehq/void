package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/aelpxy/misuDB/config"
	"github.com/aelpxy/misuDB/structs"
	"github.com/aelpxy/misuDB/utils"
)

func main() {
	utils.LoadConfig()
	utils.LoadFromDisk()

	var wg sync.WaitGroup
	wg.Add(3)

	ticker := time.Tick(time.Second)

	log.Println("-- misuDB --")

	go func() {
		log.Println("Writing to Disk...")
		for range ticker {
			utils.SaveToDisk(config.Store)
		}
	}()

	go func() {
		log.Printf("Started expiry service (expire time: %d seconds)\n", config.Config.ExpireTime)
		defer wg.Done()
		checkExpired()
	}()

	go func() {
		log.Printf("Started HTTP service on port %s\n", config.Config.HTTPPort)
		defer wg.Done()
		http.ListenAndServe(":"+config.Config.HTTPPort, nil)
	}()

	go func() {
		listener, _ := net.Listen("tcp", ":"+config.Config.TCPPort)
		defer listener.Close()
		log.Printf("Started TCP service on port %s\n", config.Config.TCPPort)
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

	if username != config.Config.Username || password != config.Config.Password {
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

			for _, ds := range config.Store {
				if ds.Key == key {
					fmt.Fprintf(conn, "Key %s already exists\n", key)
					return
				}
			}

			config.Store = append(config.Store, structs.DataStructure{
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
			for _, ds := range config.Store {
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
			for i, ds := range config.Store {
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

			config.Store = append(config.Store[:index], config.Store[index+1:]...)

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
			for i, ds := range config.Store {
				if ds.Key == key {
					config.Store[i].Value = value
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

func checkExpired() {
	for {
		now := time.Now()

		var expiredKeys []string

		for _, ds := range config.Store {
			if ds.TTL.Before(now) {
				expiredKeys = append(expiredKeys, ds.Key)
			}
		}

		if len(expiredKeys) > 0 {
			for _, key := range expiredKeys {
				for i, ds := range config.Store {
					if ds.Key == key {
						config.Store = append(config.Store[:i], config.Store[i+1:]...)
						break
					}
				}
			}
			log.Printf("Expired keys removed: %s\n", strings.Join(expiredKeys, ", "))
		}

		time.Sleep(time.Second)
	}
}
