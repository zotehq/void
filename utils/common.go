package utils

import (
	"encoding/json"
	"log"
	"os"

	"github.com/krytonitehq/void/config"
	"github.com/krytonitehq/void/structs"
)

func LoadConfig() {
	_, err := os.Stat("config.json")
	if os.IsNotExist(err) {
		CreateDefaultConfig()
	}

	file, err := os.Open("config.json")
	if err != nil {
		log.Fatal("Error opening config file:", err)
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	err = decoder.Decode(&config.Config)
	if err != nil {
		log.Fatal("Error decoding config file:", err)
	}
}

func CreateDefaultConfig() {
	config.Config = structs.Config{
		StorePath: "store.json",
	}

	file, err := os.Create("config.json")
	if err != nil {
		log.Fatal("Error creating config file:", err)
	}
	defer file.Close()

	encoder := json.NewEncoder(file)
	err = encoder.Encode(config.Config)
	if err != nil {
		log.Fatal("Error encoding config:", err)
	}

	log.Println("Default config file created: config.json")
}

func LoadFromDisk() {
	file, err := os.Open(config.Config.StorePath)
	if err != nil {
		log.Println("No previous data found")
		return
	}
	defer file.Close()

	decoder := json.NewDecoder(file)
	err = decoder.Decode(&config.Store)
	if err != nil {
		log.Fatal("Error decoding data file:", err)
	}
}

func SaveToDisk(data []structs.DataStructure) {
	file, err := os.Create(config.Config.StorePath)
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
