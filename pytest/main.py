from pysim import start_sim, SimConfig, Point, Trajectory, Car
import requests
from tensorflow.keras.models import load_model as lm
from datetime import timedelta
import pandas as pd
import json
from datetime import timedelta

IT = 2
EP = 5

NORM_X = lm(f"./it{IT}ep{EP}/normalizer_x.keras")
NORM_Y = lm(f"./it{IT}ep{EP}/normalizer_y.keras")
MODELS = {}
def load_model(): return lm(f"./it{IT}ep{EP}/model.keras")


NROM_Y_LAYER = NORM_Y.layers[0]


data = pd.read_csv("data.csv")
data = data['route'].apply(json.loads).to_list()

cars = []

for i, raw_trajectory in enumerate(data[100:150]):
    trajectory = Trajectory()
    for [t, lat, lon] in raw_trajectory:
        trajectory.add_point(Point(lat=lat, lon=lon), timedelta(
            seconds=t - raw_trajectory[0][0]))
    cars.append(Car(trajectory))

north_east = Point(12.627589, 41.999799)
south_west = Point(12.361253, 41.774020)


def predict(idx, data):
    try:
        model = MODELS[idx]
    except KeyError:
        MODELS[idx] = load_model()
        model = MODELS[idx]
    data = NORM_X(data)
    prediction = model.predict(data)[0]

    prediction = NROM_Y_LAYER.mean.numpy() + prediction \
        * NROM_Y_LAYER.variance.numpy()**0.5
    return prediction


def main():
    r = requests.get(
        "http://localhost:8080/get_roads_in_bbox.parquet?"
        + f"lat1={north_east.lat}&lon1={north_east.lon}&"
        + f"lat2={south_west.lat}&lon2={south_west.lon}"
    )

    start_sim(
        SimConfig(north_east, south_west, map=r.content,
                  cars=cars, predict=predict,
                  predict_n=40,  # projection_to="EPSG:4806",
                  server_url="http://localhost:8080",
                  step_delta=timedelta(seconds=10)))


if __name__ == "__main__":
    main()
