from pysim import start_sim, SimConfig, Point, Trajectory
import requests
from datetime import timedelta
import pandas as pd
import json

data = pd.read_csv("data.csv")
data = data['route'].apply(json.loads).to_list()

trajectories = []

for raw_trajectory in data:
    trajectory = Trajectory()
    for [t, lat, lon] in raw_trajectory:
        trajectory.add_point(Point(lat=lat, lon=lon), timedelta(
            seconds=t - raw_trajectory[0][0]))
    trajectories.append(trajectory)


north_east = Point(12.627589, 41.999799)
south_west = Point(12.361253, 41.774020)


def main():
    r = requests.get(
        "http://localhost:8080/get_roads_in_bbox.parquet?"
        + f"lat1={north_east.lat}&lon1={north_east.lon}&"
        + f"lat2={south_west.lat}&lon2={south_west.lon}"
    )

    start_sim(SimConfig(north_east, south_west, r.content,
              trajectories, projection_to="EPSG:4806"))


if __name__ == "__main__":
    main()
