#include "Metrics.h"

#include "Config.h"
#include <math.h>

Metric metrics[3] = {
    {"CPU", 0, 0, 100},
    {"GPU", 0, 0, 100},
    {"RAM", 0, 0, 100}};

TemperatureMetric temperatures[2] = {
    {"CPU", UNKNOWN_TEMPERATURE},
    {"GPU", UNKNOWN_TEMPERATURE}};

void updateAnimation(float dt)
{
  float ease = 1.0f - expf(-ANIM_SPEED * dt);

  for (int i = 0; i < 3; i++)
  {
    metrics[i].currentValue += (metrics[i].targetValue - metrics[i].currentValue) * ease;
  }
}
