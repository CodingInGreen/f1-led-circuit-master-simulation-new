use serde::Deserialize;
use serde::ser::StdError;

#[derive(Clone, Debug, Deserialize)]
pub struct LedCoordinate {
    pub x_led: f64,
    pub y_led: f64,
}

pub fn read_coordinates() -> Result<Vec<LedCoordinate>, Box<dyn StdError>> {
    Ok(vec![
        LedCoordinate {
            x_led: 6413.0,
            y_led: 33.0,
        },
        LedCoordinate {
            x_led: 6007.0,
            y_led: 197.0,
        },
        LedCoordinate {
            x_led: 5652.0,
            y_led: 444.0,
        },
        LedCoordinate {
            x_led: 5431.0,
            y_led: 822.0,
        },
        LedCoordinate {
            x_led: 5727.0,
            y_led: 1143.0,
        },
        LedCoordinate {
            x_led: 6141.0,
            y_led: 1268.0,
        },
        LedCoordinate {
            x_led: 6567.0,
            y_led: 1355.0,
        },
        LedCoordinate {
            x_led: 6975.0,
            y_led: 1482.0,
        },
        LedCoordinate {
            x_led: 7328.0,
            y_led: 1738.0,
        },
        LedCoordinate {
            x_led: 7369.0,
            y_led: 2173.0,
        },
        LedCoordinate {
            x_led: 7024.0,
            y_led: 2448.0,
        },
        LedCoordinate {
            x_led: 6592.0,
            y_led: 2505.0,
        },
        LedCoordinate {
            x_led: 6159.0,
            y_led: 2530.0,
        },
        LedCoordinate {
            x_led: 5725.0,
            y_led: 2525.0,
        },
        LedCoordinate {
            x_led: 5288.0,
            y_led: 2489.0,
        },
        LedCoordinate {
            x_led: 4857.0,
            y_led: 2434.0,
        },
        LedCoordinate {
            x_led: 4429.0,
            y_led: 2356.0,
        },
        LedCoordinate {
            x_led: 4004.0,
            y_led: 2249.0,
        },
        LedCoordinate {
            x_led: 3592.0,
            y_led: 2122.0,
        },
        LedCoordinate {
            x_led: 3181.0,
            y_led: 1977.0,
        },
        LedCoordinate {
            x_led: 2779.0,
            y_led: 1812.0,
        },
        LedCoordinate {
            x_led: 2387.0,
            y_led: 1624.0,
        },
        LedCoordinate {
            x_led: 1988.0,
            y_led: 1453.0,
        },
        LedCoordinate {
            x_led: 1703.0,
            y_led: 1779.0,
        },
        LedCoordinate {
            x_led: 1271.0,
            y_led: 1738.0,
        },
        LedCoordinate {
            x_led: 1189.0,
            y_led: 1314.0,
        },
        LedCoordinate {
            x_led: 1257.0,
            y_led: 884.0,
        },
        LedCoordinate {
            x_led: 1333.0,
            y_led: 454.0,
        },
        LedCoordinate {
            x_led: 1409.0,
            y_led: 25.0,
        },
        LedCoordinate {
            x_led: 1485.0,
            y_led: -405.0,
        },
        LedCoordinate {
            x_led: 1558.0,
            y_led: -835.0,
        },
        LedCoordinate {
            x_led: 1537.0,
            y_led: -1267.0,
        },
        LedCoordinate {
            x_led: 1208.0,
            y_led: -1555.0,
        },
        LedCoordinate {
            x_led: 779.0,
            y_led: -1606.0,
        },
        LedCoordinate {
            x_led: 344.0,
            y_led: -1604.0,
        },
        LedCoordinate {
            x_led: -88.0,
            y_led: -1539.0,
        },
        LedCoordinate {
            x_led: -482.0,
            y_led: -1346.0,
        },
        LedCoordinate {
            x_led: -785.0,
            y_led: -1038.0,
        },
        LedCoordinate {
            x_led: -966.0,
            y_led: -644.0,
        },
        LedCoordinate {
            x_led: -1015.0,
            y_led: -206.0,
        },
        LedCoordinate {
            x_led: -923.0,
            y_led: 231.0,
        },
        LedCoordinate {
            x_led: -762.0,
            y_led: 650.0,
        },
        LedCoordinate {
            x_led: -591.0,
            y_led: 1078.0,
        },
        LedCoordinate {
            x_led: -423.0,
            y_led: 1497.0,
        },
        LedCoordinate {
            x_led: -254.0,
            y_led: 1915.0,
        },
        LedCoordinate {
            x_led: -86.0,
            y_led: 2329.0,
        },
        LedCoordinate {
            x_led: 83.0,
            y_led: 2744.0,
        },
        LedCoordinate {
            x_led: 251.0,
            y_led: 3158.0,
        },
        LedCoordinate {
            x_led: 416.0,
            y_led: 3574.0,
        },
        LedCoordinate {
            x_led: 588.0,
            y_led: 3990.0,
        },
        LedCoordinate {
            x_led: 755.0,
            y_led: 4396.0,
        },
        LedCoordinate {
            x_led: 920.0,
            y_led: 4804.0,
        },
        LedCoordinate {
            x_led: 1086.0,
            y_led: 5212.0,
        },
        LedCoordinate {
            x_led: 1250.0,
            y_led: 5615.0,
        },
        LedCoordinate {
            x_led: 1418.0,
            y_led: 6017.0,
        },
        LedCoordinate {
            x_led: 1583.0,
            y_led: 6419.0,
        },
        LedCoordinate {
            x_led: 1909.0,
            y_led: 6702.0,
        },
        LedCoordinate {
            x_led: 2306.0,
            y_led: 6512.0,
        },
        LedCoordinate {
            x_led: 2319.0,
            y_led: 6071.0,
        },
        LedCoordinate {
            x_led: 2152.0,
            y_led: 5660.0,
        },
        LedCoordinate {
            x_led: 1988.0,
            y_led: 5255.0,
        },
        LedCoordinate {
            x_led: 1853.0,
            y_led: 4836.0,
        },
        LedCoordinate {
            x_led: 1784.0,
            y_led: 4407.0,
        },
        LedCoordinate {
            x_led: 1779.0,
            y_led: 3971.0,
        },
        LedCoordinate {
            x_led: 1605.0,
            y_led: 3569.0,
        },
        LedCoordinate {
            x_led: 1211.0,
            y_led: 3375.0,
        },
        LedCoordinate {
            x_led: 811.0,
            y_led: 3188.0,
        },
        LedCoordinate {
            x_led: 710.0,
            y_led: 2755.0,
        },
        LedCoordinate {
            x_led: 1116.0,
            y_led: 2595.0,
        },
        LedCoordinate {
            x_led: 1529.0,
            y_led: 2717.0,
        },
        LedCoordinate {
            x_led: 1947.0,
            y_led: 2848.0,
        },
        LedCoordinate {
            x_led: 2371.0,
            y_led: 2946.0,
        },
        LedCoordinate {
            x_led: 2806.0,
            y_led: 2989.0,
        },
        LedCoordinate {
            x_led: 3239.0,
            y_led: 2946.0,
        },
        LedCoordinate {
            x_led: 3665.0,
            y_led: 2864.0,
        },
        LedCoordinate {
            x_led: 4092.0,
            y_led: 2791.0,
        },
        LedCoordinate {
            x_led: 4523.0,
            y_led: 2772.0,
        },
        LedCoordinate {
            x_led: 4945.0,
            y_led: 2886.0,
        },
        LedCoordinate {
            x_led: 5331.0,
            y_led: 3087.0,
        },
        LedCoordinate {
            x_led: 5703.0,
            y_led: 3315.0,
        },
        LedCoordinate {
            x_led: 6105.0,
            y_led: 3484.0,
        },
        LedCoordinate {
            x_led: 6538.0,
            y_led: 3545.0,
        },
        LedCoordinate {
            x_led: 6969.0,
            y_led: 3536.0,
        },
        LedCoordinate {
            x_led: 7402.0,
            y_led: 3511.0,
        },
        LedCoordinate {
            x_led: 7831.0,
            y_led: 3476.0,
        },
        LedCoordinate {
            x_led: 8241.0,
            y_led: 3335.0,
        },
        LedCoordinate {
            x_led: 8549.0,
            y_led: 3025.0,
        },
        LedCoordinate {
            x_led: 8703.0,
            y_led: 2612.0,
        },
        LedCoordinate {
            x_led: 8662.0,
            y_led: 2173.0,
        },
        LedCoordinate {
            x_led: 8451.0,
            y_led: 1785.0,
        },
        LedCoordinate {
            x_led: 8203.0,
            y_led: 1426.0,
        },
        LedCoordinate {
            x_led: 7973.0,
            y_led: 1053.0,
        },
        LedCoordinate {
            x_led: 7777.0,
            y_led: 664.0,
        },
        LedCoordinate {
            x_led: 7581.0,
            y_led: 275.0,
        },
        LedCoordinate {
            x_led: 7274.0,
            y_led: -35.0,
        },
        LedCoordinate {
            x_led: 6839.0,
            y_led: -46.0,
        },
    ])
}