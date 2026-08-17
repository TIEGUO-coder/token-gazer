export type OpportunityDemo = {
  foundSignal: string;
  petLine: string;
  title: string;
  routeMapStatus: string;
  confirmedStatus: string;
  nextAction: string;
  plan: Array<{
    title: string;
    detail: string;
  }>;
};

const sharedPlan: OpportunityDemo["plan"] = [
  {
    title: "Pet scouts",
    detail: "Collect signals from OSS, content, services, and template markets, then bring back the most promising clue.",
  },
  {
    title: "Owner approves",
    detail: "Decide whether this is worth acting on: clear demand, small first version, and enough moving parts for agents.",
  },
  {
    title: "Grill-me breaks it down",
    detail: "Stress-test the opportunity into target users, pain points, deliverables, non-goals, and first-version scope.",
  },
  {
    title: "Move into MAH",
    detail: "Put the approved scope into a route map and assign research, creation, launch, and review tasks.",
  },
  {
    title: "Ship and self-evolve",
    detail: "Use feedback to keep adjusting the next step. This is not a one-time suggestion; it keeps moving.",
  },
];

export const opportunityDemos: OpportunityDemo[] = [
  {
    foundSignal: "Ding! I brought back something that might make money.",
    petLine: "This is not an idea bookmark. It is a small experiment worth grilling, planning, and executing.",
    title: "AI Resource Pack Shop",
    routeMapStatus: "Waiting for owner approval",
    confirmedStatus: "Approved: worth acting on",
    nextAction: "Next: send it to Grill-me, then turn the result into a MAH route map.",
    plan: sharedPlan,
  },
  {
    foundSignal: "I found another lead. This one smells like paid demand.",
    petLine: "Creators keep asking for repeatable launch assets. This could become a tiny paid workflow.",
    title: "Launch Kit Generator",
    routeMapStatus: "Waiting for owner approval",
    confirmedStatus: "Approved: worth acting on",
    nextAction: "Next: let Grill-me pressure-test the buyer, scope, and first launch kit.",
    plan: sharedPlan,
  },
  {
    foundSignal: "New clue! I found a boring task people might pay to avoid.",
    petLine: "This is a service-style automation opportunity: small, useful, and easy to test with one niche.",
    title: "Invoice Cleanup Assistant",
    routeMapStatus: "Waiting for owner approval",
    confirmedStatus: "Approved: worth acting on",
    nextAction: "Next: use Grill-me to narrow the buyer and hand the smallest workflow to MAH.",
    plan: sharedPlan,
  },
];

export const opportunityDemo = opportunityDemos[0];
