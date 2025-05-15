from pysim import start_sim, SimConfig, Point, Trajectory, Car
import requests
from tensorflow.keras.models import load_model as lm
from datetime import timedelta
import pandas as pd
import json
from datetime import timedelta


MODEL = lm("./model.keras")


data = pd.read_csv("data.csv")
data = data['route'].apply(json.loads).to_list()

cars = []

for raw_trajectory in data[:10]:
    trajectory = Trajectory()
    for [t, lat, lon] in raw_trajectory:
        trajectory.add_point(Point(lat=lat, lon=lon), timedelta(
            seconds=t - raw_trajectory[0][0]))
    cars.append(Car(trajectory))

north_east = Point(12.627589, 41.999799)
south_west = Point(12.361253, 41.774020)


def main():
    r = requests.get(
        "http://localhost:8080/get_roads_in_bbox.parquet?"
        + f"lat1={north_east.lat}&lon1={north_east.lon}&"
        + f"lat2={south_west.lat}&lon2={south_west.lon}"
    )

    start_sim(
        SimConfig(north_east, south_west, map=r.content,
                  cars=cars, predict=lambda data: MODEL.predict(data),
                  predict_n=10, projection_to="EPSG:4806",
                  step_delta=timedelta(seconds=10)))


if __name__ == "__main__":
    main()
