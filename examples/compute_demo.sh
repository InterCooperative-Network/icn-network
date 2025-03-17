#!/bin/bash

# Color codes for better readability
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}==================================================${NC}"
echo -e "${BLUE}ICN Network Distributed Compute Demo${NC}"
echo -e "${BLUE}==================================================${NC}"

# Create directories for demo
echo -e "\n${YELLOW}Creating demo directories...${NC}"
mkdir -p data_storage
mkdir -p compute_workspace
mkdir -p data_input
mkdir -p data_output
mkdir -p demo_did
mkdir -p demo_credentials

# Clean up any previous demo files
rm -rf data_storage/* compute_workspace/* data_input/* data_output/* demo_did/* demo_credentials/*

# Generate test input data
echo -e "\n${YELLOW}Generating test input data...${NC}"
echo "London,20.5,62,1012.2" > data_input/weather_data_1.csv
echo "Paris,22.1,58,1013.5" >> data_input/weather_data_1.csv
echo "Berlin,19.8,65,1011.8" >> data_input/weather_data_1.csv
echo "Rome,27.3,45,1010.2" >> data_input/weather_data_1.csv
echo "Madrid,26.9,40,1009.8" >> data_input/weather_data_1.csv

# Create a simple data processing script
echo -e "\n${YELLOW}Creating data processing scripts...${NC}"

cat > data_input/process_weather.py << EOL
#!/usr/bin/env python3
# A simple script to process weather data
import sys
import csv
from statistics import mean

def celsius_to_fahrenheit(celsius):
    return (celsius * 9/5) + 32

def process_weather_data(input_file, output_file):
    data = []
    
    # Read input data
    with open(input_file, 'r') as f:
        reader = csv.reader(f)
        for row in reader:
            city, temp_c, humidity, pressure = row
            temp_c = float(temp_c)
            humidity = float(humidity)
            pressure = float(pressure)
            
            # Convert Celsius to Fahrenheit
            temp_f = celsius_to_fahrenheit(temp_c)
            
            # Calculate heat index (simplified formula)
            heat_index = 0.5 * (temp_f + 61.0 + ((temp_f - 68.0) * 1.2) + (humidity * 0.094))
            
            # Store processed data
            data.append({
                'city': city,
                'temp_c': temp_c,
                'temp_f': temp_f,
                'humidity': humidity,
                'pressure': pressure,
                'heat_index': heat_index
            })
    
    # Calculate averages
    avg_temp_c = mean([item['temp_c'] for item in data])
    avg_temp_f = mean([item['temp_f'] for item in data])
    avg_humidity = mean([item['humidity'] for item in data])
    avg_pressure = mean([item['pressure'] for item in data])
    avg_heat_index = mean([item['heat_index'] for item in data])
    
    # Write output file
    with open(output_file, 'w') as f:
        f.write("Weather Data Analysis Results\n")
        f.write("============================\n\n")
        
        f.write("City Details:\n")
        for item in data:
            f.write(f"- {item['city']}: {item['temp_c']:.1f}°C / {item['temp_f']:.1f}°F, " +
                   f"Humidity: {item['humidity']:.1f}%, Pressure: {item['pressure']:.1f} hPa, " +
                   f"Heat Index: {item['heat_index']:.1f}°F\n")
        
        f.write("\nSummary Statistics:\n")
        f.write(f"Average Temperature: {avg_temp_c:.1f}°C / {avg_temp_f:.1f}°F\n")
        f.write(f"Average Humidity: {avg_humidity:.1f}%\n")
        f.write(f"Average Pressure: {avg_pressure:.1f} hPa\n")
        f.write(f"Average Heat Index: {avg_heat_index:.1f}°F\n")

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} input_file output_file")
        sys.exit(1)
    
    input_file = sys.argv[1]
    output_file = sys.argv[2]
    process_weather_data(input_file, output_file)
    print(f"Processed weather data from {input_file} to {output_file}")
EOL

chmod +x data_input/process_weather.py

# Also create a simple image processing script using ImageMagick
cat > data_input/process_image.sh << EOL
#!/bin/bash
# A simple script to process images using ImageMagick

if [ \$# -ne 2 ]; then
    echo "Usage: \$0 input_image output_image"
    exit 1
fi

input_image=\$1
output_image=\$2

# Apply some simple effects - resize, add border, and apply sepia tone
convert "\$input_image" -resize 800x600 -border 10x10 -bordercolor "#333333" -sepia-tone 80% "\$output_image"

echo "Processed image from \$input_image to \$output_image"
EOL

chmod +x data_input/process_image.sh

# Initialize storage system
echo -e "\n${YELLOW}Initializing storage system...${NC}"
icn-cli storage init --path ./data_storage --encrypted

# Initialize the compute environment
echo -e "\n${YELLOW}Initializing compute environment...${NC}"
icn-cli compute init --workspace ./compute_workspace --federation demo-fed

# Create DID documents for our users
echo -e "\n${YELLOW}Creating DID documents...${NC}"

cat > demo_did/data_scientist_did.json << EOL
{
  "id": "did:icn:data_scientist",
  "controller": "did:icn:data_scientist",
  "verification_method": [
    {
      "id": "did:icn:data_scientist#key-1",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:icn:data_scientist",
      "public_key_base58": "z6MkhaHgzg5sMnmJioDXNkZCVKTBNXbf1J96zNhCZuCmNF7q"
    }
  ],
  "authentication": [
    "did:icn:data_scientist#key-1"
  ],
  "service": [
    {
      "id": "did:icn:data_scientist#compute",
      "type": "ICNCompute",
      "service_endpoint": "https://compute.icn.network/data_scientist"
    }
  ]
}
EOL

# Initialize identity storage
echo -e "\n${YELLOW}Initializing identity storage...${NC}"
icn-cli identity-storage init --path ./data_storage --federation demo-fed

# Register DIDs in the system
echo -e "\n${YELLOW}Registering DIDs in the system...${NC}"
echo -e "${GREEN}Registering Data Scientist's DID...${NC}"
icn-cli identity-storage register-did --did "did:icn:data_scientist" --document demo_did/data_scientist_did.json --federation demo-fed

# Map DIDs to member IDs
echo -e "\n${YELLOW}Mapping DIDs to member IDs...${NC}"
icn-cli identity-storage map-did-to-member --did "did:icn:data_scientist" --member-id "data_scientist" --federation demo-fed

# Create verifiable credentials
echo -e "\n${YELLOW}Creating verifiable credentials...${NC}"

# Data processing credential for data scientist
cat > demo_credentials/data_processing_credential.json << EOL
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://www.w3.org/2018/credentials/examples/v1"
  ],
  "id": "credential:data_processing",
  "type": ["VerifiableCredential", "ComputeCredential"],
  "issuer": "did:icn:issuer",
  "issuanceDate": "2023-01-01T00:00:00Z",
  "expirationDate": "2033-01-01T00:00:00Z",
  "credentialSubject": {
    "id": "did:icn:data_scientist",
    "role": "DataScientist",
    "permissions": ["data_processing", "ml_training"],
    "resource_access": "standard",
    "clearance": "level-3"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2023-01-01T00:00:00Z",
    "verificationMethod": "did:icn:issuer#key-1",
    "proofPurpose": "assertionMethod",
    "jws": "eyJhbGciOiJFZERTQSIsImI2NCI6ZmFsc2UsImNyaXQiOlsiYjY0Il19..YtqjEYnFENT7fNW-COD0HAACxeuQxPKAmp4nIl8jYyO8DRb9F2ZrX1jfISf6LlKTaAECtQeJsF0k0rZVEoieCw"
  }
}
EOL

# Initialize credential storage
echo -e "\n${YELLOW}Initializing credential storage...${NC}"
icn-cli credential-storage init --path ./data_storage --federation demo-fed

# Register credentials
echo -e "\n${YELLOW}Registering credentials...${NC}"
echo -e "${GREEN}Registering Data Processing credential...${NC}"
icn-cli credential-storage register-credential --credential demo_credentials/data_processing_credential.json --federation demo-fed

# Create credential-based access rules
echo -e "\n${YELLOW}Creating credential-based access rules...${NC}"

echo -e "${GREEN}Creating data processing access rule...${NC}"
icn-cli credential-storage create-access-rule \
  --did "did:icn:data_scientist" \
  --challenge "timestamp=1621500000" \
  --signature "data_scientist_signature" \
  --pattern "*" \
  --credential-types "ComputeCredential" \
  --attributes '{"role": "DataScientist"}' \
  --permissions "read,write" \
  --federation demo-fed

# Store input files in storage
echo -e "\n${YELLOW}Storing input files in storage...${NC}"

echo -e "${GREEN}Storing weather data...${NC}"
icn-cli credential-storage store-file \
  --did "did:icn:data_scientist" \
  --challenge "timestamp=1621500010" \
  --signature "data_scientist_signature" \
  --credential-id "credential:data_processing" \
  --file data_input/weather_data_1.csv \
  --key "input/weather_data_1.csv" \
  --federation demo-fed

echo -e "${GREEN}Storing weather processing script...${NC}"
icn-cli credential-storage store-file \
  --did "did:icn:data_scientist" \
  --challenge "timestamp=1621500011" \
  --signature "data_scientist_signature" \
  --credential-id "credential:data_processing" \
  --file data_input/process_weather.py \
  --key "scripts/process_weather.py" \
  --federation demo-fed

# Submit a compute job
echo -e "\n${YELLOW}Submitting a compute job for weather data processing...${NC}"
JOB_ID=$(icn-cli compute process-data \
  --did "did:icn:data_scientist" \
  --challenge "timestamp=1621500020" \
  --signature "data_scientist_signature" \
  --credential-id "credential:data_processing" \
  --name "Weather Data Processing" \
  --command "python3" \
  --args "/workspace/scripts/process_weather.py,/workspace/input/weather_data_1.csv,/workspace/output/weather_analysis.txt" \
  --input-files "input/weather_data_1.csv:input/weather_data_1.csv,scripts/process_weather.py:scripts/process_weather.py" \
  --output-files "output/weather_analysis.txt:results/weather_analysis.txt" \
  --federation demo-fed | grep "ID:" | awk '{print $3}')

echo -e "${GREEN}Job submitted with ID: ${JOB_ID}${NC}"

# Wait for job to complete
echo -e "\n${YELLOW}Waiting for job to complete...${NC}"
sleep 3  # In a real system, we would poll for job status

# Upload job outputs to storage
echo -e "\n${YELLOW}Uploading job outputs to storage...${NC}"
icn-cli compute upload-job-outputs \
  --did "did:icn:data_scientist" \
  --challenge "timestamp=1621500030" \
  --signature "data_scientist_signature" \
  --credential-id "credential:data_processing" \
  --job-id "${JOB_ID}" \
  --federation demo-fed

# Retrieve job logs
echo -e "\n${YELLOW}Retrieving job logs...${NC}"
icn-cli compute get-job-logs \
  --did "did:icn:data_scientist" \
  --challenge "timestamp=1621500040" \
  --signature "data_scientist_signature" \
  --credential-id "credential:data_processing" \
  --job-id "${JOB_ID}" \
  --federation demo-fed

# Retrieve job status
echo -e "\n${YELLOW}Checking job status...${NC}"
icn-cli compute get-job-status \
  --did "did:icn:data_scientist" \
  --challenge "timestamp=1621500050" \
  --signature "data_scientist_signature" \
  --credential-id "credential:data_processing" \
  --job-id "${JOB_ID}" \
  --federation demo-fed

# Retrieve processed data from storage
echo -e "\n${YELLOW}Retrieving processed data from storage...${NC}"
icn-cli credential-storage get-file \
  --did "did:icn:data_scientist" \
  --challenge "timestamp=1621500060" \
  --signature "data_scientist_signature" \
  --credential-id "credential:data_processing" \
  --key "results/weather_analysis.txt" \
  --output "data_output/weather_analysis.txt" \
  --federation demo-fed

# Display the processed data
echo -e "\n${YELLOW}Displaying processed data:${NC}"
cat data_output/weather_analysis.txt

echo -e "\n${BLUE}==================================================${NC}"
echo -e "${GREEN}Distributed Compute Demo Completed Successfully${NC}"
echo -e "${BLUE}==================================================${NC}"

echo -e "\n${YELLOW}Clean up demo files? (y/n)${NC}"
read -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}Cleaning up demo files...${NC}"
    rm -rf data_storage compute_workspace data_input data_output demo_did demo_credentials
    echo -e "${GREEN}Cleanup complete.${NC}"
fi 