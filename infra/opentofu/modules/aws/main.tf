# EP-036 AWS node module main (SPEC-016).

terraform {
  required_version = ">= 1.6, < 2.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

resource "aws_instance" "nexus_node" {
  ami           = var.ami_id
  instance_type = var.instance_type

  # Bootstrap identity: the compute fabric binds exact-target readback to
  # this resource identity (SPEC-016 exact resource identity first-class).
  tags = {
    Name = var.node_name
  }
}

output "instance_id" {
  description = "Exact-target AWS resource identity for fabric readback."
  value       = aws_instance.nexus_node.id
}

output "public_ip" {
  description = "Reachability address for later verification (not readiness)."
  value       = aws_instance.nexus_node.public_ip
}
