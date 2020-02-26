package io.casperlabs.node.configuration

import java.io.File
import java.nio.file.{Path, Paths}

import eu.timepit.refined._
import eu.timepit.refined.numeric._
import eu.timepit.refined.api.Refined
import com.google.protobuf.ByteString
import io.casperlabs.comm.discovery.Node
import io.casperlabs.comm.discovery.NodeUtils.NodeWithoutChainId
import org.scalacheck.{Arbitrary, Gen}
import izumi.logstage.api.{Log => IzLog}
import scala.concurrent.duration._

trait ArbitraryImplicits {
  implicit val pathGen: Arbitrary[Path] = Arbitrary {
    for {
      n     <- Gen.choose(1, 10)
      paths <- Gen.listOfN(n, Gen.listOfN(3, Gen.alphaNumChar)).map(_.flatten.mkString(""))
    } yield Paths.get("/", paths.mkString(File.pathSeparator))
  }

  //Needed to pass through CLI options parsing
  implicit val nonEmptyStringGen: Arbitrary[String] = Arbitrary {
    for {
      n   <- Gen.choose(1, 100)
      seq <- Gen.listOfN(n, Gen.alphaLowerChar)
    } yield seq.mkString("")
  }

  //There is no way expressing explicit 'false' using CLI options.
  implicit val booleanGen: Arbitrary[Boolean] = Arbitrary {
    Gen.const(true)
  }

  // Got into trouble with values like -4.587171438322464E-226
  implicit val doubleGen: Arbitrary[Double] = Arbitrary {
    Gen.oneOf(0.1, 0.5, 1.0, 1.5, 2.0, 10.0)
  }

  implicit val nodeGen: Arbitrary[NodeWithoutChainId] = Arbitrary {
    for {
      n       <- Gen.choose(1, 100)
      bytes   <- Gen.listOfN(n, Gen.choose(Byte.MinValue, Byte.MaxValue))
      id      = ByteString.copyFrom(bytes.toArray)
      host    <- Gen.listOfN(n, Gen.alphaNumChar)
      tcpPort <- Gen.posNum[Int]
      udpPort <- Gen.posNum[Int]
    } yield NodeWithoutChainId(Node(id, host.mkString(""), tcpPort, udpPort, ByteString.EMPTY))
  }

  // There are some comparison problems with default generator
  implicit val finiteDurationGen: Arbitrary[FiniteDuration] = Arbitrary {
    for {
      n <- Gen.choose(0, Int.MaxValue)
    } yield FiniteDuration(n.toLong, MILLISECONDS)
  }

  implicit val positiveIntGen: Arbitrary[Refined[Int, Positive]] = Arbitrary {
    for {
      n <- Gen.choose(1, Int.MaxValue)
    } yield refineV[Positive](n).right.get
  }

  implicit val nonNegativeIntGen: Arbitrary[Refined[Int, NonNegative]] = Arbitrary {
    for {
      n <- Gen.choose(0, Int.MaxValue)
    } yield refineV[NonNegative](n).right.get
  }

  implicit val gte1DoubleGen: Arbitrary[Refined[Double, GreaterEqual[W.`1.0`.T]]] = Arbitrary {
    for {
      d <- Gen.choose(1.0, 10.0)
    } yield refineV[GreaterEqual[W.`1.0`.T]](d).right.get
  }

  implicit val gte0DoubleGen: Arbitrary[Refined[Double, GreaterEqual[W.`0.0`.T]]] = Arbitrary {
    for {
      d <- Gen.choose(0.0, 10.0)
    } yield refineV[GreaterEqual[W.`0.0`.T]](d).right.get
  }

  implicit val gt0lte1DoubleGen
      : Arbitrary[Refined[Double, Interval.OpenClosed[W.`0.0`.T, W.`1.0`.T]]] = Arbitrary {
    for {
      d <- Gen.choose(0.0, 1.0).filter(_ > 0)
    } yield refineV[Interval.OpenClosed[W.`0.0`.T, W.`1.0`.T]](d).right.get
  }

  implicit val levelGen: Arbitrary[IzLog.Level] = Arbitrary {
    Gen.oneOf(List(IzLog.Level.Debug, IzLog.Level.Info, IzLog.Level.Error, IzLog.Level.Warn))
  }
}
