# Kubernetes Setup

@id: infra.k8s
@status: in-progress
@labels: infrastructure, devops, p0
@created: 2025-10-20
@owner: devops-team
@estimate: 3 weeks

## Description

Kubernetes cluster setup on AWS EKS for production workloads. Multi-region deployment with automatic failover and blue-green deployments.

Cluster specifications:
- 3 availability zones for high availability
- Auto-scaling: 5-50 nodes based on load
- Node types: t3.large for general workloads, c5.xlarge for compute-intensive

## Tasks

- [x] Cluster provisioning
  - Terraform for infrastructure as code
  - GitOps approach for cluster config
  - [x] Create EKS cluster with Terraform
  - [x] Configure node groups
  - [x] Set up cluster autoscaler
  - [x] Configure pod security policies
- [ ] Networking setup
  - Calico for network policies
  - AWS Load Balancer Controller
  - Service mesh evaluation (Istio vs Linkerd)
  - [x] Install Calico CNI
  - [ ] Configure ingress controller
  - [ ] Set up service mesh
  - [ ] Configure network policies
- [ ] Storage configuration
  - EBS CSI driver for persistent volumes
  - S3 for object storage
  - [x] Install EBS CSI driver
  - [ ] Configure storage classes
  - [ ] Set up backup solution (Velero)
- [ ] Security hardening #security
  - RBAC policies with least privilege
  - Pod security standards (restricted)
  - Secrets management with AWS Secrets Manager
  - [ ] Configure RBAC policies
  - [ ] Set up pod security standards
  - [ ] Integrate secrets management
  - [ ] Enable audit logging
- [ ] Deployment automation
  - Helm for package management
  - ArgoCD for GitOps deployments
  - Blue-green deployment strategy
  - [ ] Install Helm
  - [ ] Set up ArgoCD
  - [ ] Create deployment pipelines
  - [ ] Configure rollback procedures
