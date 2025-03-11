# ICN Network Glossary of Terms

This glossary provides definitions for the specialized terminology used throughout the ICN Network codebase and documentation.

## A

**Adaptive Governance**  
A governance system that evolves based on outcomes and analysis, using machine learning and simulation to improve policies over time.

**Address Space**  
The range of possible addresses in the overlay network, typically following an IPv6-like structure.

**Allocation**  
The assignment of a resource to a specific user or cooperative for a defined period.

**Amount**  
A value representing a quantity of mutual credit, used in economic transactions.

**Assembly**  
See **Cooperative Assembly**.

**Authentication**  
The process of verifying the identity of a user, system, or entity, often utilizing DIDs and verifiable credentials.

## B

**Blinding Factor**  
A random value used in Pedersen commitments to hide transaction amounts while maintaining verifiability.

**Bulletproofs**  
A non-interactive zero-knowledge proof protocol that enables efficient range proofs.

## C

**Capability**  
A functionality that can be enabled or disabled in a node, allowing for flexible deployment across different hardware.

**Circuit**  
In onion routing, a pre-established path through multiple nodes for private communication.

**Clearing**  
The process of settling balances between federations or accounts to reduce gross obligations to net obligations.

**Commitment**  
A cryptographic primitive that allows committing to a value while keeping it hidden, used in confidential transactions.

**Committee**  
A specialized group within a Cooperative Assembly focused on a specific area of governance or expertise.

**Confidential Transaction**  
A transaction where the amount is hidden using zero-knowledge proofs while ensuring the transaction is valid.

**Consensus**  
The process by which nodes in the network agree on the state of the system.

**Cooperative**  
An autonomous association of persons united voluntarily to meet common economic, social, and cultural needs through jointly owned and democratically controlled enterprise.

**Cooperative Assembly**  
The primary decision-making body within the ICN political framework, consisting of delegates from member federations who vote on proposals and policies.

**Credit Line**  
A mutual credit relationship between two accounts, defining the maximum credit that can be extended.

**Credit Graph**  
The network of credit relationships between accounts, represented as a directed graph.

## D

**DAG (Directed Acyclic Graph)**  
A data structure used for consensus where transactions form a directed graph without cycles, enabling parallel processing.

**Delegate**  
An individual representing their federation in a Cooperative Assembly, with the authority to vote on proposals and participate in governance.

**Delegation Chain**  
A series of delegations of voting power from the original rights-holder through one or more delegates, implemented as part of the liquid democracy model.

**DID (Decentralized Identifier)**  
A globally unique identifier that doesn't require a centralized registration authority, core to the ICN identity system.

**DisputeMethod**  
The approach used to resolve conflicts within the cooperative system, such as mediation, arbitration, PeerJury, ExpertPanel, or ConsensusCircle.

**DisputeResolutionSystem**  
The framework for resolving disputes without relying on nation-state legal systems, including methods, arbiters, and appeal processes.

**DSL (Domain-Specific Language)**  
A specialized language for expressing governance rules and policies in the ICN.

## E

**Economic Action**  
An operation within the economic system, often triggered by political decisions, such as resource allocation or budget setting.

**Economic Engine**  
The component that manages the economic activities of the ICN Network, including mutual credit, resource allocation, and transactions.

**Economic System**  
The ICN subsystem that handles mutual credit, transactions, and resource exchange between cooperatives.

**Edge Node**  
A node operating at the network periphery, often with limited resources but providing local connectivity.

**Emergency Declaration**  
A formal announcement of a crisis situation requiring special allocation of resources and coordination between political and economic systems.

**Enforcement Mechanism**  
Methods to ensure compliance with cooperative legal principles and rights guarantees through economic and social means rather than state force.

## F

**Federation**  
A group of cooperatives that share a common namespace, governance, and trust relationships, forming a cooperative network.

**Federation Exchange**  
The system that enables economic activity between different federations.

**Federation Relationship**  
The formal connection between federations, categorized as Core, Partner, or Affiliated, determining the level of trust and resource sharing.

**FederationMember**  
A cooperative that belongs to a federation, with a specific status, trust score, and role within that federation.

**FederationMembershipStatus**  
The current standing of a cooperative within a federation: Probationary, Active, Suspended, or Expelled.

## G

**Governance**  
The system of rules, practices, and processes by which cooperatives are directed and controlled.

**Governance Model**  
The structure and principles guiding decision-making within a federation or assembly.

**Governance VM**  
The virtual machine that executes governance policies in a secure, sandboxed environment.

**GovernanceEngine**  
The component responsible for executing governance rules, managing voting processes, and implementing democratic decisions.

## I

**ICN (Intercooperative Network)**  
The complete system described in this documentation, comprising all subsystems and components.

**Identity Component**  
The core module in every ICN node responsible for DID management, credential verification, and identity operations.

**Identity System**  
The subsystem responsible for DIDs, credentials, and privacy-preserving identity verification.

**Implementation Plan**  
A structured approach for executing a proposal that has been approved through the political process.

**ImplementationStatus**  
The current state of a proposal's execution: NotStarted, InProgress, Completed, or Failed.

**Impact Assessment**  
An evaluation of the potential effects of a proposal, categorized by severity (High, Medium, Low) and affected areas.

**Integration Layer**  
The component that coordinates and synchronizes operations between different subsystems, particularly between political and economic frameworks.

## K

**Key Rotation**  
The periodic changing of cryptographic keys to mitigate the risk of key compromise.

## L

**Legal Framework**  
The cooperative alternative to state legal systems, including principles, dispute resolution mechanisms, and precedents.

**Legal Principle**  
A fundamental rule or standard in the cooperative legal framework, serving as a foundation for decisions and dispute resolution.

**Liquid Democracy**  
A form of democratic governance where voting power can be delegated to representatives on specific issues, combining direct and representative democracy.

## M

**Mesh Network**  
A network topology where nodes connect directly to as many other nodes as possible, enabling resilient local connectivity.

**MobilityPassport**  
A digital document that enables workers and refugees to move between federations while maintaining their rights and protections.

**Mutual Credit**  
A non-speculative economic system where credit is created when an account goes negative, balanced by another account going positive.

## N

**Network Layer**  
The subsystem handling communication between nodes, including transport security, overlay networking, and mesh capabilities.

**Node**  
A participant in the ICN network that implements one or more ICN subsystems.

## O

**Onion Routing**  
A technique for anonymous communication where messages are encrypted in layers, like an onion.

**Overlay Network**  
A virtual network built on top of existing network infrastructure, enabling ICN-specific routing and addressing.

**OverlayAddress**  
A unique identifier for a node in the overlay network, often mapped to a DID.

**OversightMechanism**  
The democratic controls placed on security teams and enforcement functions, including review periods and transparency requirements.

## P

**PassportStatus**  
The current state of a MobilityPassport: Active, Suspended, Expired, or Revoked.

**PassportType**  
The category of a MobilityPassport: Worker, Refugee, Delegate, or SecurityTeam.

**Path Transaction**  
A transaction that flows through multiple credit lines to reach its destination.

**Pedersen Commitment**  
A cryptographic commitment scheme used in confidential transactions to hide amounts.

**PolicyDomain**  
A specific area of governance focus, such as labor rights, resource allocation, or environmental standards.

**Political Engine**  
The component that manages the political activities of the ICN Network, including assemblies, proposals, voting, security protocols, and legal frameworks.

**Post-Quantum Cryptography**  
Cryptographic algorithms believed to be secure against attacks by quantum computers.

**Privacy**  
The protection of information and identity in the ICN, implemented through zero-knowledge proofs, ring signatures, and secure multi-party computation.

**Proposal**  
A formal suggestion for policy, resource allocation, or other decision requiring approval through the governance process.

**ProposalStatus**  
The current state of a proposal in the governance process: Draft, Proposed, Voting, Passed, Rejected, Implemented, or Failed.

**ProposalType**  
The category of a proposal, such as LaborRights, ResourceAllocation, DisputeResolution, etc., determining which processes and thresholds apply.

## Q

**Quadratic Voting**  
A collective decision-making procedure where voting power scales as the square root of the number of votes, reducing the power of large stakeholders.

## R

**Range Proof**  
A zero-knowledge proof that a value lies within a specific range, used to prove transaction amounts are positive without revealing them.

**Reputation**  
A measure of trustworthiness in the ICN, affecting credit limits and governance weight.

**Resource**  
Any shareable asset managed by the ICN, including computing resources, storage, network, and physical assets.

**Resource Coordination System**  
The subsystem responsible for registering, allocating, and coordinating resources among cooperatives.

**RightsGuarantee**  
A specific right ensured to the holder of a MobilityPassport, including the type of right, description, enforcement mechanism, and appeal process.

**RightType**  
The category of right guaranteed to an individual, such as Labor, Housing, Healthcare, Education, etc.

**Ring Signature**  
A cryptographic signature that specifies a group of possible signers without revealing which member actually produced the signature.

## S

**Secure Channel**  
An encrypted communication path between nodes, typically using TLS 1.3 or WireGuard.

**Secure Multi-Party Computation (MPC)**  
A cryptographic technique allowing multiple parties to jointly compute a function over their inputs while keeping those inputs private.

**SecurityProtocol**  
The framework for maintaining safety and security within the cooperative system through democratic oversight rather than state enforcement.

**SecurityTeam**  
A group responsible for addressing security concerns within a federation, subject to democratic oversight and accountability.

**Security Domain**  
A classification for information and operations based on their security requirements.

**Selective Disclosure**  
The ability to reveal only specific attributes from a credential without exposing the entire credential.

**SNARK (Succinct Non-interactive Argument of Knowledge)**  
A form of zero-knowledge proof that is small in size and quick to verify.

**STARK (Scalable Transparent Argument of Knowledge)**  
A transparent, post-quantum secure zero-knowledge proof system.

## T

**Time Slot**  
A period of time for which a resource is allocated or an operation is scheduled.

**Timeline**  
The schedule for implementing a proposal: Immediate, Scheduled, or Phased.

**Transaction**  
An economic exchange between accounts recorded in the system.

**TransportSecurityManager**  
The component responsible for securing network communications.

## V

**Verifiable Credential**  
A cryptographically secure digital credential that can be verified without contacting the issuer.

**VM (Virtual Machine)**  
In ICN, the sandboxed execution environment for governance policies.

**Vote**  
A recorded decision on a proposal, including the voter's DID, federation, vote type, weight, and cryptographic signature.

**VoteType**  
The nature of a vote: Approve, Reject, Abstain, or Delegate.

**Voting System**  
The component responsible for collecting, tallying, and verifying votes on governance proposals.

## W

**WireGuard**  
A modern VPN protocol used in the ICN for secure peer-to-peer connections.

## Z

**Zero-Knowledge Proof (ZKP)**  
A cryptographic method by which one party can prove to another that a statement is true without revealing any information beyond the validity of the statement itself.

**zkp-SNARK**  
A specific type of zero-knowledge proof used in the ICN for efficient verification.

**zkp-STARK**  
A transparent, post-quantum secure zero-knowledge proof system used in the ICN. 