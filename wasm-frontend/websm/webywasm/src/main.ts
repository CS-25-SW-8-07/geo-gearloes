import { WebHandle, StateHandle, insert_into_index } from "rustywasm"

// const sleep = async (time: number) => await new Promise(r => setTimeout(r, time));


function pullLocation(stateHandle: StateHandle) {
  navigator.geolocation.watchPosition(pos => stateHandle.add_point(pos.coords.longitude, pos.coords.latitude));
}

const ENDPOINT = "get_roads_in_bbox.parquet";

const lon1 = 9.833100;
const lat1 = 57.114689;

const lon2 = 10.012082
const lat2 = 56.999152;

function getData(state: StateHandle) {
  fetch(`/${ENDPOINT}?lon1=${lon1}&lat1=${lat1}&lon2=${lon2}&lat2=${lat2}`)
    .then(r => r.bytes())
    .then(b => insert_into_index(b, state));
}

async function main() {
  const app = document.getElementById("app");
  const stateHandle = new StateHandle();

  getData(stateHandle.clone());

  const wh = new WebHandle(stateHandle.clone());

  if (app instanceof HTMLCanvasElement) {
    wh.start(app).catch(console.error);
    pullLocation(stateHandle.clone());
  } else {
    console.error("App is not a canvas");
  }
}

main();

