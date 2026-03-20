variable "environment_name" {
  type        = string
  default     = "openclaw"
  description = "Name prefix for all resources"

  validation {
    condition     = can(regex("^[a-z0-9-]+$", var.environment_name)) && length(var.environment_name) <= 24
    error_message = "Must be lowercase alphanumeric with hyphens, max 24 chars."
  }
}

variable "instance_type" {
  type        = string
  default     = "t4g.xlarge"
  description = "EC2 instance type (must be arm64/Graviton)"

  validation {
    condition     = contains(["t4g.medium", "t4g.large", "t4g.xlarge", "t4g.2xlarge", "m7g.medium", "m7g.large", "m7g.xlarge", "c7g.large", "c7g.xlarge"], var.instance_type)
    error_message = "Must be a supported arm64/Graviton instance type."
  }
}

variable "vpc_cidr" {
  type        = string
  default     = "10.0.0.0/16"
  description = "CIDR block for the VPC"
}

variable "public_subnet_cidr" {
  type        = string
  default     = "10.0.1.0/24"
  description = "CIDR block for the public subnet"
}

variable "ssh_allowed_cidr" {
  type        = string
  default     = "127.0.0.1/32"
  description = "CIDR block allowed to SSH. Default disables SSH (use SSM instead)."
}

variable "root_volume_size" {
  type        = number
  default     = 40
  description = "Root EBS volume size in GB (gp3)"

  validation {
    condition     = var.root_volume_size >= 20 && var.root_volume_size <= 200
    error_message = "Must be between 20 and 200."
  }
}

variable "data_volume_size" {
  type        = number
  default     = 80
  description = "Data EBS volume size in GB (gp3) mounted at /mnt/ebs-data"

  validation {
    condition     = var.data_volume_size >= 20 && var.data_volume_size <= 500
    error_message = "Must be between 20 and 500."
  }
}

variable "key_pair_name" {
  type        = string
  default     = ""
  description = "EC2 key pair name for SSH access (leave blank to skip)"
}

variable "openclaw_gateway_port" {
  type        = number
  default     = 18789
  description = "Port for the OpenClaw gateway service"

  validation {
    condition     = var.openclaw_gateway_port >= 1024 && var.openclaw_gateway_port <= 65535
    error_message = "Must be between 1024 and 65535."
  }
}

variable "bedrock_region" {
  type        = string
  default     = "us-east-1"
  description = "AWS region for Bedrock API calls"

  validation {
    condition     = contains(["us-east-1", "us-west-2", "eu-west-1", "eu-central-1", "eu-north-1", "ap-northeast-1", "ap-southeast-1"], var.bedrock_region)
    error_message = "Must be a supported Bedrock region."
  }
}

variable "default_model" {
  type        = string
  default     = "us.anthropic.claude-opus-4-6-v1"
  description = "Default Bedrock model ID for OpenClaw"
}

variable "model_mode" {
  type        = string
  default     = "bedrock"
  description = "Model access mode: litellm, api-key, or bedrock"

  validation {
    condition     = contains(["litellm", "api-key", "bedrock"], var.model_mode)
    error_message = "Must be litellm, api-key, or bedrock."
  }
}

variable "litellm_base_url" {
  type        = string
  default     = ""
  description = "LiteLLM proxy base URL (used when model_mode=litellm)"
}

variable "litellm_api_key" {
  type        = string
  default     = ""
  sensitive   = true
  description = "LiteLLM virtual API key (used when model_mode=litellm)"
}

variable "litellm_model" {
  type        = string
  default     = "claude-opus-4-6"
  description = "Default model alias on the LiteLLM proxy"
}

variable "provider_api_key" {
  type        = string
  default     = ""
  sensitive   = true
  description = "Direct provider API key (used when model_mode=api-key)"
}

variable "bootstrap_script_url" {
  type        = string
  default     = "https://raw.githubusercontent.com/inceptionstack/loki-bootstrap/main/deploy/openclaw-bootstrap.sh"
  description = "URL to the bootstrap script"
}

variable "request_quota_increases" {
  type        = string
  default     = "false"
  description = "Automatically request Bedrock quota increases at deploy time"

  validation {
    condition     = contains(["true", "false"], var.request_quota_increases)
    error_message = "Must be true or false."
  }
}
