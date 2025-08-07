//! Trajectory recorder implementation

use crate::error::{Result, TrajectoryError};
use crate::trajectory::TrajectoryEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;

/// Records execution trajectories for debugging and analysis
pub struct TrajectoryRecorder {
    entries: RwLock<Vec<TrajectoryEntry>>,
    file_path: Option<PathBuf>,
    auto_save: bool,
}

/// Complete trajectory data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    /// Metadata about the trajectory
    pub metadata: TrajectoryMetadata,
    
    /// All trajectory entries
    pub entries: Vec<TrajectoryEntry>,
}

/// Metadata for a trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryMetadata {
    /// Unique identifier for this trajectory
    pub id: String,
    
    /// When the trajectory was started
    pub started_at: DateTime<Utc>,
    
    /// When the trajectory was completed (if completed)
    pub completed_at: Option<DateTime<Utc>>,
    
    /// Version of the trajectory format
    pub version: String,
    
    /// Agent that created this trajectory
    pub agent_type: String,
    
    /// Task that was being executed
    pub task: Option<String>,
    
    /// Whether the task was successful
    pub success: Option<bool>,
    
    /// Total number of steps
    pub total_steps: usize,
    
    /// Total duration in milliseconds
    pub duration_ms: Option<u64>,
}

impl TrajectoryRecorder {
    /// Create a new trajectory recorder
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            file_path: None,
            auto_save: false,
        }
    }
    
    /// Create a trajectory recorder that saves to a file
    pub fn with_file<P: AsRef<Path>>(path: P) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            file_path: Some(path.as_ref().to_path_buf()),
            auto_save: true,
        }
    }
    
    /// Create a trajectory recorder with auto-generated filename
    pub fn with_auto_filename() -> Self {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("trajectory_{}.json", timestamp);
        
        // Create trajectories directory if it doesn't exist
        let trajectories_dir = Path::new("trajectories");
        if !trajectories_dir.exists() {
            std::fs::create_dir_all(trajectories_dir).ok();
        }
        
        let path = trajectories_dir.join(filename);
        Self::with_file(path)
    }
    
    /// Record a trajectory entry
    pub async fn record(&self, entry: TrajectoryEntry) -> Result<()> {
        {
            let mut entries = self.entries.write().await;
            entries.push(entry);
        }
        
        if self.auto_save {
            self.save().await?;
        }
        
        Ok(())
    }
    
    /// Get all recorded entries
    pub async fn get_entries(&self) -> Vec<TrajectoryEntry> {
        self.entries.read().await.clone()
    }
    
    /// Get the number of recorded entries
    pub async fn entry_count(&self) -> usize {
        self.entries.read().await.len()
    }
    
    /// Save the trajectory to file
    pub async fn save(&self) -> Result<()> {
        if let Some(path) = &self.file_path {
            let trajectory = self.build_trajectory().await;
            let json = serde_json::to_string_pretty(&trajectory)
                .map_err(|e| TrajectoryError::RecordingFailed {
                    message: format!("Failed to serialize trajectory: {}", e),
                })?;
            
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            
            fs::write(path, json).await?;
        }
        
        Ok(())
    }
    
    /// Load a trajectory from file
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Trajectory> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(TrajectoryError::LoadFailed {
                path: path.to_string_lossy().to_string(),
            }.into());
        }
        
        let content = fs::read_to_string(path).await?;
        let trajectory: Trajectory = serde_json::from_str(&content)
            .map_err(|_| TrajectoryError::InvalidFormat)?;
        
        Ok(trajectory)
    }
    
    /// Build a complete trajectory from recorded entries
    async fn build_trajectory(&self) -> Trajectory {
        let entries = self.entries.read().await.clone();
        
        let started_at = entries.first()
            .map(|e| e.timestamp)
            .unwrap_or_else(Utc::now);
        
        let completed_at = entries.last()
            .map(|e| e.timestamp);
        
        let duration_ms = completed_at.map(|end| {
            (end - started_at).num_milliseconds() as u64
        });
        
        // Extract task and success from entries
        let mut task = None;
        let mut success = None;
        
        for entry in &entries {
            match &entry.entry_type {
                crate::trajectory::EntryType::TaskStart { task: t, .. } => {
                    task = Some(t.clone());
                }
                crate::trajectory::EntryType::TaskComplete { success: s, .. } => {
                    success = Some(*s);
                }
                _ => {}
            }
        }
        
        let metadata = TrajectoryMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            started_at,
            completed_at,
            version: "1.0".to_string(),
            agent_type: "trae_agent".to_string(),
            task,
            success,
            total_steps: entries.len(),
            duration_ms,
        };
        
        Trajectory { metadata, entries }
    }
    
    /// Clear all recorded entries
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }
    
    /// Get the file path if set
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }
}

impl Default for TrajectoryRecorder {
    fn default() -> Self {
        Self::new()
    }
}
