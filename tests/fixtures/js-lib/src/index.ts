import { greet } from "./utils";

export function main(name: string = "World"): string {
  return greet(name);
}

if (require.main === module) {
  console.log(main());
}
