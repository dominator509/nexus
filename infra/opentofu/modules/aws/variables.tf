# EP-036 AWS node module variables (SPEC-016).

variable "region" {
  description = "AWS region slug (canonical shape, e.g. us-east-1)."
  type        = string

  validation {
    condition     = can(regex("^[a-z]{2}-[a-z]+-[0-9]$", var.region))
    error_message = "region must be a canonical AWS region slug."
  }
}

variable "instance_type" {
  description = "EC2 instance type for the Nexus node."
  type        = string

  validation {
    condition     = length(var.instance_type) > 0
    error_message = "instance_type must not be empty."
  }
}

variable "node_name" {
  description = "Compute node name (fabric registry identity)."
  type        = string

  validation {
    condition     = length(var.node_name) > 0 && length(var.node_name) <= 128
    error_message = "node_name must be 1..=128 characters."
  }
}

variable "ami_id" {
  description = "Base AMI for the Nexus bootstrap image."
  type        = string

  validation {
    condition     = length(var.ami_id) > 0
    error_message = "ami_id must not be empty."
  }
}

variable "ssh_key_name" {
  description = "Existing EC2 key pair name used as bootstrap identity."
  type        = string

  validation {
    condition     = length(var.ssh_key_name) > 0
    error_message = "ssh_key_name must not be empty."
  }
}
