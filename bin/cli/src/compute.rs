use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;
use async_trait::async_trait;
use uuid::Uuid;
use std::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::credential_storage::{CredentialStorageService, CredentialProvider, VerifiableCredential};
use crate::identity_storage::{IdentityProvider, IdentityStorageService};

/// Status of a compute job in the ICN Network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComputeJobStatus {
    /// Job has been submitted but not yet scheduled
    Submitted,
    /// Job is queued for execution
    Queued,
    /// Job is currently running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed with error
    Failed { error: String },
    /// Job was cancelled
    Cancelled,
    /// Job timed out
    TimedOut,
}

/// Resource requirements for a compute job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResources {
    /// Required CPU cores
    pub cpu_cores: u32,
    /// Required memory in MB
    pub memory_mb: u32,
    /// Required GPU memory in MB (if any)
    pub gpu_memory_mb: Option<u32>,
    /// Maximum execution time in seconds
    pub max_execution_time_sec: u64,
}

impl Default for ComputeResources {
    fn default() -> Self {
        Self {
            cpu_cores: 1,
            memory_mb: 512,
            gpu_memory_mb: None,
            max_execution_time_sec: 300,
        }
    }
}

/// A compute job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeJob {
    /// Unique job identifier
    pub id: String,
    /// Job name
    pub name: String,
    /// Job description
    pub description: Option<String>,
    /// Job owner (DID)
    pub owner_did: String,
    /// Federation this job belongs to
    pub federation: String,
    /// Command to execute
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Input files from storage (storage key to local path mapping)
    pub input_files: HashMap<String, String>,
    /// Output files to storage (local path to storage key mapping)
    pub output_files: HashMap<String, String>,
    /// Required compute resources
    pub resources: ComputeResources,
    /// Credential ID required for execution
    pub credential_id: Option<String>,
    /// Current job status
    pub status: ComputeJobStatus,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl ComputeJob {
    /// Create a new compute job
    pub fn new(
        name: &str, 
        owner_did: &str, 
        federation: &str, 
        command: &str,
        resources: ComputeResources
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            owner_did: owner_did.to_string(),
            federation: federation.to_string(),
            command: command.to_string(),
            args: Vec::new(),
            env_vars: HashMap::new(),
            input_files: HashMap::new(),
            output_files: HashMap::new(),
            resources,
            credential_id: None,
            status: ComputeJobStatus::Submitted,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add an argument to the job command
    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Add multiple arguments to the job command
    pub fn with_args(mut self, args: Vec<&str>) -> Self {
        for arg in args {
            self.args.push(arg.to_string());
        }
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }

    /// Add an input file from storage
    pub fn with_input_file(mut self, storage_key: &str, local_path: &str) -> Self {
        self.input_files.insert(storage_key.to_string(), local_path.to_string());
        self
    }

    /// Add an output file to storage
    pub fn with_output_file(mut self, local_path: &str, storage_key: &str) -> Self {
        self.output_files.insert(local_path.to_string(), storage_key.to_string());
        self
    }

    /// Set a description for the job
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set a credential ID required for execution
    pub fn with_credential(mut self, credential_id: &str) -> Self {
        self.credential_id = Some(credential_id.to_string());
        self
    }
}

/// Trait defining a compute executor that can run jobs
#[async_trait]
pub trait ComputeExecutor {
    /// Submit a job for execution
    async fn submit_job(&self, job: ComputeJob) -> Result<String>;
    
    /// Get job status
    async fn get_job_status(&self, job_id: &str) -> Result<ComputeJobStatus>;
    
    /// Get job details
    async fn get_job(&self, job_id: &str) -> Result<Option<ComputeJob>>;
    
    /// List jobs
    async fn list_jobs(&self, owner_did: Option<&str>, status: Option<ComputeJobStatus>) -> Result<Vec<ComputeJob>>;
    
    /// Cancel a job
    async fn cancel_job(&self, job_id: &str) -> Result<()>;
    
    /// Get job logs
    async fn get_job_logs(&self, job_id: &str) -> Result<String>;
}

/// Local compute executor that runs jobs on the local machine
pub struct LocalComputeExecutor {
    /// Base directory for job workspaces
    workspace_dir: PathBuf,
    /// Jobs database
    jobs: HashMap<String, ComputeJob>,
    /// Logs storage
    logs: HashMap<String, String>,
}

impl LocalComputeExecutor {
    /// Create a new local compute executor
    pub fn new<P: AsRef<Path>>(workspace_dir: P) -> Result<Self> {
        let workspace_path = workspace_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&workspace_path)?;
        
        Ok(Self {
            workspace_dir: workspace_path,
            jobs: HashMap::new(),
            logs: HashMap::new(),
        })
    }
    
    /// Get the workspace directory for a job
    fn get_job_workspace(&self, job_id: &str) -> PathBuf {
        self.workspace_dir.join(job_id)
    }
    
    /// Create the job workspace directory
    async fn create_job_workspace(&self, job_id: &str) -> Result<PathBuf> {
        let workspace = self.get_job_workspace(job_id);
        tokio::fs::create_dir_all(&workspace).await?;
        Ok(workspace)
    }
    
    /// Execute a job
    async fn execute_job(&mut self, job: ComputeJob) -> Result<ComputeJobStatus> {
        let job_id = job.id.clone();
        let workspace = self.create_job_workspace(&job_id).await?;
        
        // Update job status to Running
        let mut updated_job = job.clone();
        updated_job.status = ComputeJobStatus::Running;
        updated_job.updated_at = Utc::now();
        self.jobs.insert(job_id.clone(), updated_job.clone());
        
        // Prepare command
        let mut cmd = Command::new(&job.command);
        cmd.args(&job.args);
        cmd.current_dir(&workspace);
        cmd.env_clear(); // Start with a clean environment
        
        // Add environment variables
        for (key, value) in &job.env_vars {
            cmd.env(key, value);
        }
        
        // Execute with timeout
        let timeout = std::time::Duration::from_secs(job.resources.max_execution_time_sec);
        let execution = tokio::time::timeout(timeout, cmd.output()).await;
        
        // Process result
        let status = match execution {
            Ok(output_result) => {
                match output_result {
                    Ok(output) => {
                        // Store logs
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let log_content = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);
                        self.logs.insert(job_id.clone(), log_content);
                        
                        if output.status.success() {
                            ComputeJobStatus::Completed
                        } else {
                            let code = output.status.code().unwrap_or(-1);
                            ComputeJobStatus::Failed(format!("Process exited with code {}", code))
                        }
                    },
                    Err(e) => ComputeJobStatus::Failed(format!("Execution error: {}", e)),
                }
            },
            Err(_) => ComputeJobStatus::TimedOut,
        };
        
        // Update job status
        let mut final_job = updated_job;
        final_job.status = status.clone();
        final_job.updated_at = Utc::now();
        self.jobs.insert(job_id, final_job);
        
        Ok(status)
    }
}

#[async_trait]
impl ComputeExecutor for LocalComputeExecutor {
    async fn submit_job(&self, job: ComputeJob) -> Result<String> {
        let job_id = job.id.clone();
        // In a real implementation, we would add the job to a queue here
        // For simplicity, we're just storing it and returning the ID
        Ok(job_id)
    }
    
    async fn get_job_status(&self, job_id: &str) -> Result<ComputeJobStatus> {
        match self.jobs.get(job_id) {
            Some(job) => Ok(job.status.clone()),
            None => Err(anyhow::anyhow!("Job not found: {}", job_id)),
        }
    }
    
    async fn get_job(&self, job_id: &str) -> Result<Option<ComputeJob>> {
        Ok(self.jobs.get(job_id).cloned())
    }
    
    async fn list_jobs(&self, owner_did: Option<&str>, status: Option<ComputeJobStatus>) -> Result<Vec<ComputeJob>> {
        let mut jobs = Vec::new();
        
        for job in self.jobs.values() {
            // Filter by owner if specified
            if let Some(owner) = owner_did {
                if job.owner_did != owner {
                    continue;
                }
            }
            
            // Filter by status if specified
            if let Some(ref filter_status) = status {
                if &job.status != filter_status {
                    continue;
                }
            }
            
            jobs.push(job.clone());
        }
        
        // Sort by creation time, newest first
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(jobs)
    }
    
    async fn cancel_job(&self, job_id: &str) -> Result<()> {
        // In a real implementation, we would cancel a running job
        // For this example, we'll just return an error if the job doesn't exist
        if !self.jobs.contains_key(job_id) {
            return Err(anyhow::anyhow!("Job not found: {}", job_id));
        }
        
        // For now, we would mark the job as cancelled in a real implementation
        Ok(())
    }
    
    async fn get_job_logs(&self, job_id: &str) -> Result<String> {
        match self.logs.get(job_id) {
            Some(logs) => Ok(logs.clone()),
            None => {
                // If job exists but no logs, return empty logs
                if self.jobs.contains_key(job_id) {
                    Ok(String::new())
                } else {
                    Err(anyhow::anyhow!("Job not found: {}", job_id))
                }
            }
        }
    }
}

/// Credential-based compute service that integrates with storage
pub struct CredentialComputeService<I: IdentityProvider, C: CredentialProvider> {
    /// Base directory for job workspaces
    workspace_dir: PathBuf,
    /// Federation name
    federation: String,
    /// Credential storage service for authentication and access control
    credential_storage: CredentialStorageService<C, I>,
    /// Compute executor for running jobs
    executor: Box<dyn ComputeExecutor + Send + Sync>,
    /// Jobs database file path
    jobs_db_path: PathBuf,
    /// Jobs database
    jobs: HashMap<String, ComputeJob>,
}

impl<I: IdentityProvider + Send + Sync + 'static, C: CredentialProvider + Send + Sync + 'static> CredentialComputeService<I, C> {
    /// Create a new credential-based compute service
    pub async fn new(
        federation: &str,
        workspace_dir: impl Into<PathBuf>,
        credential_storage: CredentialStorageService<C, I>,
    ) -> Result<Self> {
        let workspace_path = workspace_dir.into();
        std::fs::create_dir_all(&workspace_path)?;
        
        let executor = Box::new(LocalComputeExecutor::new(&workspace_path)?);
        let jobs_db_path = workspace_path.join("jobs.json");
        
        let jobs = if jobs_db_path.exists() {
            let content = tokio::fs::read_to_string(&jobs_db_path).await?;
            serde_json::from_str(&content)?
        } else {
            HashMap::new()
        };
        
        Ok(Self {
            workspace_dir: workspace_path,
            federation: federation.to_string(),
            credential_storage,
            executor,
            jobs_db_path,
            jobs,
        })
    }
    
    /// Save jobs database to disk
    async fn save_jobs(&self) -> Result<()> {
        let content = serde_json::to_string(&self.jobs)?;
        tokio::fs::write(&self.jobs_db_path, content).await?;
        Ok(())
    }
    
    /// Authenticate with DID and check credential
    async fn authenticate(
        &self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
    ) -> Result<()> {
        // First authenticate the DID
        let member_id = self.credential_storage.get_identity_storage_mut()
            .authenticate_did(did, challenge, signature)
            .await?;
            
        // If a credential ID is provided, verify it
        if let Some(cred_id) = credential_id {
            let credential = self.credential_storage.get_credential_provider()
                .resolve_credential(cred_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Credential not found: {}", cred_id))?;
                
            // Verify the credential
            let status = self.credential_storage.get_credential_provider()
                .verify_credential(&credential)
                .await?;
                
            // Check if the credential is valid
            match status {
                crate::credential_storage::CredentialVerificationStatus::Verified => {
                    // Check if the credential belongs to the authenticated DID
                    if credential.credentialSubject.id != did {
                        return Err(anyhow::anyhow!("Credential subject does not match authenticated DID"));
                    }
                },
                status => {
                    return Err(anyhow::anyhow!("Invalid credential: {:?}", status));
                }
            }
        }
        
        Ok(())
    }
    
    /// Submit a compute job with credential authentication
    pub async fn submit_job(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        job: ComputeJob,
    ) -> Result<String> {
        // Authenticate the DID and verify credential
        self.authenticate(did, challenge, signature, credential_id).await?;
        
        // Check if the job belongs to the authenticated DID
        if job.owner_did != did {
            return Err(anyhow::anyhow!("Job owner DID does not match authenticated DID"));
        }
        
        // Submit the job
        let job_id = self.executor.submit_job(job.clone()).await?;
        
        // Store the job
        self.jobs.insert(job_id.clone(), job);
        
        // Save jobs to disk
        self.save_jobs().await?;
        
        Ok(job_id)
    }
    
    /// Get job status with credential authentication
    pub async fn get_job_status(
        &self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        job_id: &str,
    ) -> Result<ComputeJobStatus> {
        // Authenticate the DID and verify credential
        self.authenticate(did, challenge, signature, credential_id).await?;
        
        // Check if the job exists and belongs to the authenticated DID
        match self.jobs.get(job_id) {
            Some(job) => {
                if job.owner_did != did {
                    return Err(anyhow::anyhow!("Job owner DID does not match authenticated DID"));
                }
                
                // Get the job status
                self.executor.get_job_status(job_id).await
            },
            None => Err(anyhow::anyhow!("Job not found: {}", job_id)),
        }
    }
    
    /// Get job details with credential authentication
    pub async fn get_job(
        &self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        job_id: &str,
    ) -> Result<ComputeJob> {
        // Authenticate the DID and verify credential
        self.authenticate(did, challenge, signature, credential_id).await?;
        
        // Check if the job exists and belongs to the authenticated DID
        match self.jobs.get(job_id) {
            Some(job) => {
                if job.owner_did != did {
                    return Err(anyhow::anyhow!("Job owner DID does not match authenticated DID"));
                }
                
                // Get the job details
                match self.executor.get_job(job_id).await? {
                    Some(job) => Ok(job),
                    None => Err(anyhow::anyhow!("Job not found in executor: {}", job_id)),
                }
            },
            None => Err(anyhow::anyhow!("Job not found: {}", job_id)),
        }
    }
    
    /// List jobs with credential authentication
    pub async fn list_jobs(
        &self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        status: Option<ComputeJobStatus>,
    ) -> Result<Vec<ComputeJob>> {
        // Authenticate the DID and verify credential
        self.authenticate(did, challenge, signature, credential_id).await?;
        
        // List jobs for the authenticated DID
        self.executor.list_jobs(Some(did), status).await
    }
    
    /// Cancel a job with credential authentication
    pub async fn cancel_job(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        job_id: &str,
    ) -> Result<()> {
        // Authenticate the DID and verify credential
        self.authenticate(did, challenge, signature, credential_id).await?;
        
        // Check if the job exists and belongs to the authenticated DID
        match self.jobs.get_mut(job_id) {
            Some(job) => {
                if job.owner_did != did {
                    return Err(anyhow::anyhow!("Job owner DID does not match authenticated DID"));
                }
                
                // Cancel the job
                self.executor.cancel_job(job_id).await?;
                
                // Update job status to Cancelled
                job.status = ComputeJobStatus::Cancelled;
                job.updated_at = Utc::now();
                
                // Save jobs to disk
                self.save_jobs().await?;
                
                Ok(())
            },
            None => Err(anyhow::anyhow!("Job not found: {}", job_id)),
        }
    }
    
    /// Get job logs with credential authentication
    pub async fn get_job_logs(
        &self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        job_id: &str,
    ) -> Result<String> {
        // Authenticate the DID and verify credential
        self.authenticate(did, challenge, signature, credential_id).await?;
        
        // Check if the job exists and belongs to the authenticated DID
        match self.jobs.get(job_id) {
            Some(job) => {
                if job.owner_did != did {
                    return Err(anyhow::anyhow!("Job owner DID does not match authenticated DID"));
                }
                
                // Get the job logs
                self.executor.get_job_logs(job_id).await
            },
            None => Err(anyhow::anyhow!("Job not found: {}", job_id)),
        }
    }
    
    /// Execute a data processing job that reads input files from storage and writes output files to storage
    pub async fn execute_data_processing_job(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        job: ComputeJob,
    ) -> Result<String> {
        // Authenticate the DID and verify credential
        self.authenticate(did, challenge, signature, credential_id).await?;
        
        // Create a workspace for the job
        let job_id = job.id.clone();
        let workspace = self.workspace_dir.join(&job_id);
        tokio::fs::create_dir_all(&workspace).await?;
        
        // Download input files from storage
        for (storage_key, local_path) in &job.input_files {
            let output_path = workspace.join(local_path);
            
            // Create parent directories if needed
            if let Some(parent) = output_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            
            // Get the file from storage using credential authentication
            self.credential_storage.retrieve_file(
                did,
                challenge,
                signature,
                credential_id,
                storage_key,
                &output_path,
                None,
            ).await?;
        }
        
        // Submit the job for execution
        let job_id = self.submit_job(did, challenge, signature, credential_id, job.clone()).await?;
        
        // In a real implementation, we would wait for the job to complete here
        // For this example, we're just returning the job ID immediately
        
        Ok(job_id)
    }
    
    /// Upload output files to storage after job completion
    pub async fn upload_job_outputs(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        credential_id: Option<&str>,
        job_id: &str,
    ) -> Result<()> {
        // Authenticate the DID and verify credential
        self.authenticate(did, challenge, signature, credential_id).await?;
        
        // Check if the job exists and belongs to the authenticated DID
        let job = match self.jobs.get(job_id) {
            Some(job) => {
                if job.owner_did != did {
                    return Err(anyhow::anyhow!("Job owner DID does not match authenticated DID"));
                }
                job.clone()
            },
            None => return Err(anyhow::anyhow!("Job not found: {}", job_id)),
        };
        
        // Check if the job is completed
        if job.status != ComputeJobStatus::Completed {
            return Err(anyhow::anyhow!("Job is not completed: {:?}", job.status));
        }
        
        // Upload output files to storage
        let workspace = self.workspace_dir.join(job_id);
        
        for (local_path, storage_key) in &job.output_files {
            let file_path = workspace.join(local_path);
            
            // Check if the file exists
            if !file_path.exists() {
                return Err(anyhow::anyhow!("Output file not found: {}", local_path));
            }
            
            // Store the file in storage using credential authentication
            self.credential_storage.store_file(
                did,
                challenge,
                signature,
                credential_id,
                &file_path,
                storage_key,
                job.federation == "encrypted", // Use federation name to determine if encryption is needed
            ).await?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential_storage::MockCredentialProvider;
    use crate::identity_storage::MockIdentityProvider;
    
    #[tokio::test]
    async fn test_compute_job_creation() {
        let resources = ComputeResources {
            cpu_cores: 2,
            memory_mb: 1024,
            gpu_memory_mb: None,
            max_execution_time_sec: 300,
        };
        
        let job = ComputeJob::new(
            "test-job",
            "did:icn:alice",
            "test-federation",
            "echo",
            resources,
        )
        .with_arg("Hello, world!")
        .with_env("TEST_VAR", "test-value")
        .with_input_file("input.txt", "input.txt")
        .with_output_file("output.txt", "output.txt")
        .with_description("Test job")
        .with_credential("credential:1");
        
        assert_eq!(job.name, "test-job");
        assert_eq!(job.owner_did, "did:icn:alice");
        assert_eq!(job.federation, "test-federation");
        assert_eq!(job.command, "echo");
        assert_eq!(job.args, vec!["Hello, world!"]);
        assert_eq!(job.env_vars.get("TEST_VAR"), Some(&"test-value".to_string()));
        assert_eq!(job.input_files.get("input.txt"), Some(&"input.txt".to_string()));
        assert_eq!(job.output_files.get("output.txt"), Some(&"output.txt".to_string()));
        assert_eq!(job.description, Some("Test job".to_string()));
        assert_eq!(job.credential_id, Some("credential:1".to_string()));
        assert_eq!(job.status, ComputeJobStatus::Submitted);
    }
} 