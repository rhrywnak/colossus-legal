// =============================================================================
// Create Case Node and INVOLVES Relationships
// Run this script in Neo4j Browser or via cypher-shell
// =============================================================================

// Create Case node
CREATE (c:Case {
  id: "awad-v-cfs-2011",
  title: "Marie Awad v. Catholic Family Service, et al.",
  case_number: "2011-XXXXX",
  court: "Macomb County Circuit Court",
  court_type: "Civil",
  filing_date: "2011-11-13",
  status: "Active",
  summary: "Plaintiff Marie Awad seeks declaratory relief and damages against Catholic Family Service and George Phillips for breach of fiduciary duty, fraud, and abuse of process arising from the guardianship/conservatorship of her father, Emil Awad."
});

// Create INVOLVES relationships to link parties to the case
// Plaintiff
MATCH (c:Case {id: "awad-v-cfs-2011"}), (p:Person {id: "marie-awad"})
CREATE (c)-[:INVOLVES {role: "plaintiff"}]->(p);

// Defendant - Organization
MATCH (c:Case {id: "awad-v-cfs-2011"}), (o:Organization {id: "catholic-family-service"})
CREATE (c)-[:INVOLVES {role: "defendant"}]->(o);

// Defendant - Person
MATCH (c:Case {id: "awad-v-cfs-2011"}), (p:Person {id: "george-phillips"})
CREATE (c)-[:INVOLVES {role: "defendant"}]->(p);

// Ward (other party)
MATCH (c:Case {id: "awad-v-cfs-2011"}), (p:Person {id: "emil-awad"})
CREATE (c)-[:INVOLVES {role: "ward"}]->(p);
