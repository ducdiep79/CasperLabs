package io.casperlabs.casper.highway

import cats._
import cats.implicits._
import cats.data.NonEmptyList
import io.casperlabs.crypto.hash.Blake2b256
import io.casperlabs.crypto.Keys.{PublicKey, PublicKeyBS}
import io.casperlabs.casper.consensus.{Bond, Era}
import io.casperlabs.catscontrib.MonadThrowable
import java.security.SecureRandom
import java.nio.{ByteBuffer, ByteOrder}

trait LeaderSequencer {
  def apply[F[_]: MonadThrowable](era: Era): F[LeaderFunction]
}

object LeaderSequencer extends LeaderSequencer {

  override def apply[F[_]: MonadThrowable](era: Era): F[LeaderFunction] =
    MonadThrowable[F].fromOption(
      NonEmptyList
        .fromList {
          era.bonds.filterNot(x => x.getStake.value.isEmpty || x.getStake.value == "0").toList
        }
        .map { bonds =>
          apply(era.bookingBlockHash.toByteArray, bonds)
        },
      new IllegalStateException("There must be some bonded validators in the era!")
    )

  /** Concatentate all the magic bits into a byte array,
    * padding them with zeroes on the right.
    */
  def toByteArray(bits: Seq[Boolean]): Array[Byte] = {
    val arr = Array.fill(math.ceil(bits.size / 8.0).toInt)(0)
    bits.zipWithIndex.foreach {
      case (bit, i) =>
        if (bit) {
          val a = i / 8
          val b = 7 - i % 8
          arr(a) = arr(a) | 1 << b
        }
    }
    arr.map(_.toByte)
  }

  def seed(parentSeed: Array[Byte], magicBits: Seq[Boolean]) =
    Blake2b256.hash(parentSeed ++ toByteArray(magicBits))

  /** Make a function that assigns a leader to each round, deterministically,
    * with a relative frequency based on their weight. */
  def apply(leaderSeed: Array[Byte], bonds: NonEmptyList[Bond]): LeaderFunction = {
    // Make a list of (validator, from, to) triplets.
    type ValidatorRange = (PublicKeyBS, BigInt, BigInt)

    val (validators, total) = {
      val acc = bonds
        .foldLeft(List.empty[ValidatorRange] -> BigInt(0)) {
          case ((ranges, total), bond) =>
            val key   = PublicKey(bond.validatorPublicKey)
            val stake = BigInt(bond.getStake.value)
            // This should be trivial; the auction should not allow 0 bids,
            // but if it did, there would be no way to pick between them.
            require(stake > 0, s"Bonds must be positive: $stake")
            val from = total
            val to   = total + stake
            ((key, from, to) :: ranges) -> to
        }
      // Keep the order of validator, it's coming from the block, same for everyone.
      val ranges = acc._1.reverse.toVector
      // Using BigDecimal to be able to multiply with a Double later.
      val total = BigDecimal(acc._2)
      ranges -> total
    }

    // Given a target sum of bonds, find the validator with a total cumulative weight in that range.
    def bisect(target: BigInt, i: Int = 0, j: Int = validators.size - 1): PublicKeyBS = {
      val k = (i + j) / 2
      val v = validators(k)
      // The first validator has the 0 inclusive, upper exclusive.
      if (v._2 <= target && target < v._3 || i == j) {
        v._1
      } else if (target < v._2) {
        bisect(target, i, k)
      } else {
        bisect(target, k + 1, j)
      }
    }

    (tick: Ticks) => {
      // On Linux SecureRandom uses NativePRNG, and ignores the seed.
      // Re-seeding also doesn't reset the seed, just augments it, so a new instance is required.
      // https://stackoverflow.com/questions/50107982/rhe-7-not-respecting-java-secure-random-seed
      // NODE-1095: Find a more secure, cross platform algorithm.
      val random = SecureRandom.getInstance("SHA1PRNG", "SUN")
      // Ticks need to be deterministic, so each time we have to reset the seed.
      val tickSeed = leaderSeed ++ longToBytesLittleEndian(tick)
      random.setSeed(tickSeed)
      // Pick a number between [0, 1) and use it to find a validator.
      // NODE-1096: If possible generate a random BigInt directly, without involving a Double.
      val r = BigDecimal.valueOf(random.nextDouble())
      // Integer arithmetic is supposed to be safer than Double.
      val t = (total * r).toBigInt
      // Find the first validator over the target.
      bisect(t)
    }
  }

  private def longToBytesLittleEndian(i: Long): Array[Byte] =
    ByteBuffer
      .allocate(8)
      .order(ByteOrder.LITTLE_ENDIAN)
      .putLong(i)
      .array
}
